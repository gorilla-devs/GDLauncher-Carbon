//! Integration tests for `execute_plan` + `render_audit` — the filesystem
//! and audit-formatting halves of the staging rewrite around the apply
//! planner. Deliberately does NOT drive `process_modpack_staging` itself (it
//! needs a live `App`); instead each test builds a real
//! instance-root/staging-dir tree under `tempfile`, derives a plan via
//! `apply_plan::plan` (already exhaustively table-tested on its own terms in
//! `apply_plan.rs`), and checks what `execute_plan` actually did to the
//! filesystem and what `render_audit` says about it.
//!
//! Scenarios (a)-(e) are the named TDD scenarios. The two
//! `ReplaceDisabled`/`Replace`+`Create`+`Delete` tests below them are
//! additional coverage: none of (a)-(e) exercises those `PlanAction` arms of
//! `execute_plan` against a real filesystem (the golden in (e) only exercises
//! `render_audit`), so they are covered here too rather than left untested.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::PathBuf;

use sha2::Digest as _;

use super::apply_plan::{
    self, ApplyMode, DiskState, PlanAction, PlanEntry, PlanInputs, PlanReason,
};
use super::disk_scan;
use super::packinfo::{FileHashes, PackInfo};
use super::{finish_promoted_staging, render_audit};

async fn bound_cleanup(
    options: &super::RepairMarkerFile,
    old: Option<&PackInfo>,
    target: &PackInfo,
    root: &std::path::Path,
    insensitive: bool,
) -> super::CleanupReport {
    super::apply_bound_cleanup(
        options,
        old,
        target,
        root,
        insensitive,
        options.cleanup_selections.first().map(|s| &s.context),
        None,
    )
    .await
}

// Existing cleanup controls model a fresh explicit manual selection.
async fn apply_user_cleanup(
    paths: &[String],
    old: Option<&PackInfo>,
    target: &PackInfo,
    root: &std::path::Path,
    insensitive: bool,
) -> Vec<String> {
    let mut options = super::RepairMarkerFile::default();
    for path in paths {
        if live_path(root, path).is_file() && super::normalize_cleanup_path(path).is_some() {
            let selected = cleanup_options(root, path).await;
            options.cleanup_batch = selected.cleanup_batch;
            options
                .cleanup_selections
                .extend(selected.cleanup_selections);
        }
    }
    bound_cleanup(&options, old, target, root, insensitive)
        .await
        .removed
}

async fn execute_plan(
    entries: &[PlanEntry],
    root: &std::path::Path,
    staging: &std::path::Path,
) -> anyhow::Result<Vec<PlanEntry>> {
    let mut target = super::packinfo::scan_dir(&staging.join("instance"), None).await?;
    // Older planner fixtures use synthetic hashes. Supply the real fixture
    // target for the supported pristine/no-download ReEnable executor control.
    if entries
        .iter()
        .any(|e| e.action == PlanAction::ReEnable && !target.files.contains_key(&e.path))
    {
        let live = super::packinfo::scan_dir(&root.join("instance"), None).await?;
        for entry in entries.iter().filter(|e| e.action == PlanAction::ReEnable) {
            if let Some(hashes) = live.files.get(&entry.path) {
                target
                    .files
                    .entry(entry.path.clone())
                    .or_insert_with(|| hashes.clone());
            }
        }
    }
    super::execute_plan(entries, root, staging, &target).await
}

fn hashes(seed: u8) -> FileHashes {
    FileHashes {
        sha512: [seed; 64],
        md5: [seed; 16],
    }
}

fn packinfo(entries: &[(&str, u8)]) -> PackInfo {
    PackInfo {
        files: entries
            .iter()
            .map(|(p, s)| (p.to_string(), hashes(*s)))
            .collect(),
    }
}

/// A fresh `<tmp>/instance_root/{instance,.setup/staging/instance}` tree.
/// The `TempDir` guard must stay alive for the whole test — bind it, don't
/// discard it with `_`.
fn scaffold() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let instance_root = tmp.path().join("instance_root");
    let staging_dir = instance_root.join(".setup").join("staging");
    std::fs::create_dir_all(instance_root.join("instance")).unwrap();
    std::fs::create_dir_all(staging_dir.join("instance")).unwrap();
    (tmp, instance_root, staging_dir)
}

fn live_path(instance_root: &std::path::Path, path: &str) -> PathBuf {
    instance_root.join("instance").join(&path[1..])
}

fn disabled_path(instance_root: &std::path::Path, path: &str) -> PathBuf {
    let live = live_path(instance_root, path);
    let mut name = live.file_name().unwrap().to_owned();
    name.push(".disabled");
    live.with_file_name(name)
}

fn staged_path(staging_dir: &std::path::Path, path: &str) -> PathBuf {
    staging_dir.join("instance").join(&path[1..])
}

async fn write_live(instance_root: &std::path::Path, path: &str, bytes: &[u8]) {
    let dest = live_path(instance_root, path);
    tokio::fs::create_dir_all(dest.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(dest, bytes).await.unwrap();
}

async fn write_disabled(instance_root: &std::path::Path, path: &str, bytes: &[u8]) {
    let dest = disabled_path(instance_root, path);
    tokio::fs::create_dir_all(dest.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(dest, bytes).await.unwrap();
}

async fn write_staged(staging_dir: &std::path::Path, path: &str, bytes: &[u8]) {
    let dest = staged_path(staging_dir, path);
    tokio::fs::create_dir_all(dest.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::write(dest, bytes).await.unwrap();
}

// --- (a) deleted-stays-deleted ---------------------------------------------

#[tokio::test]
async fn deleted_by_user_stays_deleted() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/a.jar";

    // Old and target both ship this path, and a fresh copy of it was staged
    // (the pack still wants it there) — but the user deleted their own copy,
    // so disk has nothing at PATH.
    let old = packinfo(&[(PATH, 1)]);
    let target = packinfo(&[(PATH, 1)]);
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();
    write_staged(&staging_dir, PATH, b"fresh-pack-bytes").await;

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Missing);

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a deleted-by-user path must plan without error");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::Keep);
    assert_eq!(entries[0].reason, PlanReason::DeletedByUser);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error on a Keep entry");

    assert!(
        !live_path(&instance_root, PATH).exists(),
        "Keep/DeletedByUser must not resurrect the file"
    );

    let audit = render_audit(&entries, &[]);
    assert!(
        audit.contains(" - /mods/a.jar: deleted by user\n"),
        "audit was:\n{audit}"
    );
}

// --- (b) damaged file under VersionChange ----------------------------------

#[tokio::test]
async fn modified_by_user_is_kept_with_both_md5s_in_the_audit() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/config/a.json";

    let old = packinfo(&[(PATH, 1)]);
    let target = packinfo(&[(PATH, 2)]);
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();
    write_staged(&staging_dir, PATH, b"new-pack-bytes").await;
    write_live(&instance_root, PATH, b"user-edited-bytes").await;

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Present { md5: hashes(9).md5 });

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a modified-by-user path must plan without error");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::Keep);
    assert_eq!(
        entries[0].reason,
        PlanReason::ModifiedByUser {
            original: hashes(1).md5,
            current: hashes(9).md5,
        }
    );

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error on a Keep entry");

    let bytes = tokio::fs::read(live_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(
        bytes, b"user-edited-bytes",
        "Keep must never touch the user's bytes"
    );

    let audit = render_audit(&entries, &[]);
    let expected_line = format!(
        " - {PATH}: modified by user\n     original md5: {}\n     current md5:  {}\n",
        hex::encode(hashes(1).md5),
        hex::encode(hashes(9).md5),
    );
    assert!(audit.contains(&expected_line), "audit was:\n{audit}");
}

// --- (d) ReEnable ------------------------------------------------------------

#[tokio::test]
async fn reenable_pristine_disabled_twin_uses_the_twins_own_bytes() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/pristine.jar";

    let old = packinfo(&[(PATH, 5)]);
    let target = packinfo(&[(PATH, 2)]);
    let staged: HashSet<String> = HashSet::new();
    write_disabled(&instance_root, PATH, b"pristine-disabled-bytes").await;

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Disabled { md5: hashes(2).md5 });

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::Repair {
            re_enable_disabled: true,
        },
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a pristine disabled twin re-enables without needing staged bytes");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::ReEnable);
    assert_eq!(entries[0].reason, PlanReason::ReEnabled);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error re-enabling a pristine twin");

    let bytes = tokio::fs::read(live_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(bytes, b"pristine-disabled-bytes");
    assert!(
        !disabled_path(&instance_root, PATH).exists(),
        "the twin must be gone once re-enabled"
    );
}

/// With nothing staged, a disabled file that holds the pack's bytes is
/// renamed to its bare name, as 2.0.40 did: the same file, not a copy, and
/// no second copy kept in `.repair-recovery` (only the transition's
/// manifest is there, until the setup finishes). Before, every re-enabled
/// mod was stored twice for good.
#[tokio::test]
async fn reenable_without_a_staged_copy_renames_the_twin_and_keeps_no_copy() {
    let (_tmp, root, staging) = scaffold();
    const PATH: &str = "/mods/pristine.jar";
    write_disabled(&root, PATH, b"pristine-disabled-bytes").await;
    #[cfg(unix)]
    let inode = {
        use std::os::unix::fs::MetadataExt;
        std::fs::metadata(disabled_path(&root, PATH)).unwrap().ino()
    };
    let entries = vec![PlanEntry {
        path: PATH.into(),
        action: PlanAction::ReEnable,
        reason: PlanReason::ReEnabled,
    }];
    let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();
    assert_eq!(outcomes, entries);
    assert_eq!(
        std::fs::read(live_path(&root, PATH)).unwrap(),
        b"pristine-disabled-bytes"
    );
    assert!(!disabled_path(&root, PATH).exists());
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        assert_eq!(
            std::fs::metadata(live_path(&root, PATH)).unwrap().ino(),
            inode,
            "renamed, not copied"
        );
    }
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(super::transition_path(&root, PATH)).unwrap())
            .unwrap();
    assert_eq!(record["completed"], true);
    assert_eq!(record["conflict"], false);
    let entry = root
        .join(".repair-recovery")
        .join(record["recovery_batch"].as_str().unwrap())
        .join(super::record_key(PATH));
    assert!(entry.join("entry.json").is_file());
    assert!(!entry.join("payload").exists(), "no second copy is kept");
}

/// A disabled file that no longer holds the pack's bytes, with nothing
/// staged, is not renamed: both names stay as they are and the change is a
/// conflict, as before.
#[tokio::test]
async fn reenable_without_a_staged_copy_keeps_a_changed_twin_disabled() {
    let (_tmp, root, staging) = scaffold();
    const PATH: &str = "/mods/changed.jar";
    write_disabled(&root, PATH, b"edited").await;
    let entries = vec![PlanEntry {
        path: PATH.into(),
        action: PlanAction::ReEnable,
        reason: PlanReason::ReEnabled,
    }];
    let target = packinfo(&[(PATH, 2)]);
    let mut notes = Vec::new();
    let outcomes = super::execute_bound_plan(
        &entries,
        &root,
        &staging,
        &target,
        None,
        &mut notes,
        None,
        &Default::default(),
    )
    .await
    .unwrap();
    assert_eq!(outcomes[0].action, PlanAction::Keep, "{notes:?}");
    assert!(!live_path(&root, PATH).exists());
    assert_eq!(
        std::fs::read(disabled_path(&root, PATH)).unwrap(),
        b"edited"
    );
}

#[tokio::test]
async fn reenable_damaged_disabled_twin_uses_staged_bytes_and_drops_the_twin() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/modified.jar";

    let old = packinfo(&[(PATH, 5)]);
    let target = packinfo(&[(PATH, 2)]);
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();
    write_disabled(&instance_root, PATH, b"damaged-disabled-bytes").await;
    write_staged(&staging_dir, PATH, b"fresh-pack-bytes").await;

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Disabled { md5: hashes(9).md5 });

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::Repair {
            re_enable_disabled: true,
        },
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a damaged disabled twin with staged bytes re-enables without error");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::ReEnable);
    assert_eq!(entries[0].reason, PlanReason::ReEnabled);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error re-enabling a damaged twin");

    let bytes = tokio::fs::read(live_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(
        bytes, b"fresh-pack-bytes",
        "a damaged twin must be re-enabled from the staged pack bytes, not its own"
    );
    assert!(
        !disabled_path(&instance_root, PATH).exists(),
        "the damaged twin must be dropped, not left behind"
    );
    assert!(
        staged_path(&staging_dir, PATH).exists(),
        "the original staged source must survive until finalization"
    );
}

#[tokio::test]
async fn reenable_errors_when_the_stale_twin_cannot_be_removed() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/locked.jar";

    let old = packinfo(&[(PATH, 5)]);
    let target = packinfo(&[(PATH, 2)]);
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();
    write_staged(&staging_dir, PATH, b"fresh-pack-bytes").await;

    // The "disabled twin" spelling is a non-empty directory rather than a
    // regular file — `remove_file` on it fails (on Unix with EISDIR/ENOTEMPTY
    // depending on platform), standing in for "twin can't be removed" (a
    // locked file in the real world) without needing real file locking in a
    // test.
    let twin = disabled_path(&instance_root, PATH);
    tokio::fs::create_dir_all(&twin).await.unwrap();
    tokio::fs::write(twin.join("keep.txt"), b"not empty")
        .await
        .unwrap();

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Disabled { md5: hashes(9).md5 });

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::Repair {
            re_enable_disabled: true,
        },
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a damaged disabled twin with staged bytes plans without error");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::ReEnable);

    let result = execute_plan(&entries, &instance_root, &staging_dir).await;
    assert!(
        result.is_err(),
        "execute_plan must surface a twin it cannot remove instead of swallowing the error"
    );

    // No audit is ever rendered for this attempt — `process_modpack_staging`
    // propagates `execute_plan`'s error with `?` before it reaches the
    // audit-writing code, so the caller can never promote or report an apply
    // that left a stale twin behind unremoved.
}

// --- extra: ReplaceDisabled (repair, no re-enable) — outside the (a)-(e)
// --- list, and the only other execute_plan arm ReEnable's tests don't
// --- already cover.

#[tokio::test]
async fn repair_replaces_a_damaged_disabled_twin_in_place() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/disabled.jar";

    let old = packinfo(&[(PATH, 5)]);
    let target = packinfo(&[(PATH, 2)]);
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();
    write_disabled(&instance_root, PATH, b"damaged-disabled-bytes").await;
    write_staged(&staging_dir, PATH, b"repaired-pack-bytes").await;

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Disabled { md5: hashes(9).md5 });

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::Repair {
            re_enable_disabled: false,
        },
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a damaged disabled twin with staged bytes repairs without error");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::ReplaceDisabled);
    assert_eq!(
        entries[0].reason,
        PlanReason::RepairOverwrote {
            original: hashes(2).md5,
            current: hashes(9).md5,
        }
    );

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error replacing a disabled twin in place");

    assert!(
        !live_path(&instance_root, PATH).exists(),
        "repairing a disabled twin must not enable it"
    );
    let bytes = tokio::fs::read(disabled_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(bytes, b"repaired-pack-bytes");
}

// --- extra: Replace + Create + Delete together under VersionChange — the
// --- three actions the (a)-(e) list also leaves unexercised against
// --- a real filesystem.

#[tokio::test]
async fn version_change_replaces_creates_and_deletes_real_files() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const REPLACE: &str = "/mods/replace.jar";
    const CREATE: &str = "/mods/create.jar";
    const DROP: &str = "/mods/drop.jar";

    let old = packinfo(&[(REPLACE, 1), (DROP, 1)]);
    let target = packinfo(&[(REPLACE, 2), (CREATE, 3)]);
    let staged: HashSet<String> = [REPLACE, CREATE].into_iter().map(String::from).collect();

    write_live(&instance_root, REPLACE, b"old-pack-bytes").await;
    write_staged(&staging_dir, REPLACE, b"new-pack-bytes").await;
    write_staged(&staging_dir, CREATE, b"created-bytes").await;
    write_live(&instance_root, DROP, b"stale-pack-bytes").await;

    let mut disk = HashMap::new();
    disk.insert(
        REPLACE.to_string(),
        DiskState::Present { md5: hashes(1).md5 },
    );
    disk.insert(DROP.to_string(), DiskState::Present { md5: hashes(1).md5 });
    // CREATE is absent from disk -> defaults to Missing.

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a fully-staged version change must not error");
    assert_eq!(entries.len(), 3);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error on Replace/Create/Delete");

    assert_eq!(
        tokio::fs::read(live_path(&instance_root, REPLACE))
            .await
            .unwrap(),
        b"new-pack-bytes"
    );
    assert_eq!(
        tokio::fs::read(live_path(&instance_root, CREATE))
            .await
            .unwrap(),
        b"created-bytes"
    );
    assert!(!live_path(&instance_root, DROP).exists());

    let audit = render_audit(&entries, &[]);
    assert!(audit.contains("\nFiles replaced:\n - /mods/replace.jar\n"));
    assert!(audit.contains("\nFiles created:\n - /mods/create.jar\n"));
    assert!(audit.contains("\nFiles deleted:\n - /mods/drop.jar\n"));
}

// --- regression: a path whose only staged copy is `.disabled`-suffixed ----
//
// `packinfo::scan_dir` strips a trailing `.disabled` when keying
// tmp-packinfo.json, so a `PlanEntry::path` is always the bare spelling —
// but a pack that ships an override disabled by default only ever stages it
// under the `.disabled` name (override extraction preserves the archive's
// own filename verbatim). Before `staged`'s key-normalisation and
// `execute_plan`'s `resolve_staged` fallback, this pair of facts meant such
// a path looked permanently unstaged to the planner: a fresh install of a
// pack containing one hit `PlanError::MissingStagedSource` and errored
// before ever reaching `.setup` cleanup, so every retry failed identically.

#[tokio::test]
async fn pack_shipped_disabled_file_creates_disabled_and_is_seen_as_disabled_afterwards() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/optional.jar";

    // The pack ships this override pre-disabled: target only ever records
    // the bare path (scan_dir strips the suffix), but the only physical
    // staged copy sits under the twin-suffixed name.
    let target = packinfo(&[(PATH, 2)]);
    write_staged(
        &staging_dir,
        &format!("{PATH}.disabled"),
        b"pack-disabled-bytes",
    )
    .await;

    // Mirrors process_modpack_staging's own staged-set normalisation: strip
    // a trailing `.disabled` before inserting, so membership checks align
    // with packinfo-style keys the same way tmp-packinfo.json's do.
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Missing);

    let entries = apply_plan::plan(PlanInputs {
        old: None,
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a pack-shipped-disabled path must plan without error, not MissingStagedSource");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::Create);
    assert_eq!(entries[0].reason, PlanReason::PackUpdate);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must resolve the .disabled-suffixed staged source");

    // (i) Lands disabled, not enabled — the pack's own default is preserved.
    assert!(
        !live_path(&instance_root, PATH).exists(),
        "a pack-shipped-disabled file must not land enabled"
    );
    let bytes = tokio::fs::read(disabled_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(bytes, b"pack-disabled-bytes");

    // (ii) A later pass's own disk scan must see it as Disabled, not
    // Missing — otherwise the very next version change hits the identical
    // MissingStagedSource error this fixes, one version later.
    let universe: BTreeSet<String> = [PATH.to_string()].into_iter().collect();
    let disk_after = disk_scan::scan_disk_state(&instance_root.join("instance"), &universe)
        .await
        .unwrap();
    assert!(
        matches!(
            disk_after.states.get(PATH),
            Some(DiskState::Disabled { .. })
        ),
        "expected the path to scan as Disabled after landing at the twin spelling, got {:?}",
        disk_after.states.get(PATH)
    );

    // Re-planning as a later version change would (same target, nothing
    // freshly staged this time) must classify it Keep/DisabledByUser, not
    // error again.
    let entries2 = apply_plan::plan(PlanInputs {
        old: Some(&target),
        target: &target,
        staged: &HashSet::new(),
        disk: &disk_after.states,
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a later pass over the same disabled twin must not error");
    assert_eq!(entries2.len(), 1);
    assert_eq!(entries2[0].action, PlanAction::Keep);
    assert_eq!(entries2[0].reason, PlanReason::DisabledByUser);
}

#[tokio::test]
async fn replace_with_pack_shipped_disabled_target_drops_the_old_enabled_copy() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/becomes-disabled.jar";

    // Previously enabled and pristine; the new version now ships this
    // override disabled by default (e.g. a mod moving from default-on to
    // default-off across a version bump).
    let old = packinfo(&[(PATH, 1)]);
    let target = packinfo(&[(PATH, 2)]);
    write_live(&instance_root, PATH, b"old-enabled-bytes").await;
    write_staged(
        &staging_dir,
        &format!("{PATH}.disabled"),
        b"new-disabled-bytes",
    )
    .await;
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Present { md5: hashes(1).md5 });

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a pristine path becoming pack-shipped-disabled must plan without error");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::Replace);
    assert_eq!(entries[0].reason, PlanReason::PackUpdate);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must resolve the .disabled-suffixed staged source");

    assert!(
        !live_path(&instance_root, PATH).exists(),
        "the old enabled copy must not survive alongside the new disabled one"
    );
    let bytes = tokio::fs::read(disabled_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(bytes, b"new-disabled-bytes");
}

#[tokio::test]
async fn resumed_apply_finishes_an_interrupted_disabled_replace_via_real_disk_scan() {
    // Simulates the exact disk state a crash (or a locked-file `remove_file`
    // failure) leaves between execute_plan's disabled-Replace rename
    // (staged -> live/PATH.disabled, already landing target's own bytes)
    // and its follow-up removal of the stale live/PATH: both spellings on
    // disk, the bare one still pristine (old bytes), staging already
    // consumed by the interrupted attempt (nothing re-staged here). Uses a
    // real `disk_scan::scan_disk_state` scan — not hand-built `DiskState` —
    // so this proves the whole pipeline (scanner surfaces the coexisting
    // twin -> planner recognizes and finishes the interrupted state ->
    // execute_plan performs exactly the Delete) works end to end, not just
    // the pure planner in isolation.
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/becomes-disabled.jar";

    // Real md5s of the actual bytes on disk below, not the synthetic
    // `hashes(seed)` helper — `disk_scan::scan_disk_state` hashes real file
    // content, so `old`/`target` have to agree with it for this to exercise
    // the same comparisons a real resumed apply would make.
    let old_md5: [u8; 16] = md5::Md5::digest(b"old-enabled-bytes").into();
    let target_md5: [u8; 16] = md5::Md5::digest(b"new-disabled-bytes").into();
    let old = PackInfo {
        files: [(
            PATH.to_string(),
            FileHashes {
                sha512: [0; 64],
                md5: old_md5,
            },
        )]
        .into_iter()
        .collect(),
    };
    let target = PackInfo {
        files: [(
            PATH.to_string(),
            FileHashes {
                sha512: [0; 64],
                md5: target_md5,
            },
        )]
        .into_iter()
        .collect(),
    };
    write_live(&instance_root, PATH, b"old-enabled-bytes").await;
    write_disabled(&instance_root, PATH, b"new-disabled-bytes").await;
    // Nothing staged — the interrupted attempt's own rename already
    // consumed it, and overrides are never re-extracted on resume.
    let staged: HashSet<String> = HashSet::new();

    let universe: BTreeSet<String> = [PATH.to_string()].into_iter().collect();
    let disk_scan::DiskScan {
        states: disk,
        coexisting_disabled_twin_md5,
    } = disk_scan::scan_disk_state(&instance_root.join("instance"), &universe)
        .await
        .unwrap();

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &coexisting_disabled_twin_md5,
    })
    .expect("a coexisting twin already matching target must finish the interrupted replace");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::Delete);
    assert_eq!(entries[0].reason, PlanReason::DisabledReplaceResumed);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must finish the interrupted replace without error");

    assert!(
        !live_path(&instance_root, PATH).exists(),
        "the stale bare copy must be deleted, finishing the interrupted replace"
    );
    let bytes = tokio::fs::read(disabled_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(
        bytes, b"new-disabled-bytes",
        "the twin, which already had target's own bytes, must be left untouched"
    );

    let audit = render_audit(&entries, &[]);
    assert!(
        audit.contains(&format!("\nFiles deleted:\n - {PATH}\n")),
        "an interrupted-replace resume must render as an honest Delete, audit was:\n{audit}"
    );
}

// --- Repair-mode execute_plan integration (plain enabled files) ---
//
// apply_plan.rs's own table tests already prove decide_repair picks the
// right (action, reason) pair for a damaged/missing enabled file; these
// prove execute_plan actually moves the right bytes into place for those
// decisions, the same gap version_change_replaces_creates_and_deletes_real_files
// closes for VersionChange. old is deliberately a third, distinct hash from
// target in each case — repair decides purely against target, never old.

#[tokio::test]
async fn repair_damaged_enabled_file_is_replaced_from_staged_bytes() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/damaged.jar";

    let old = packinfo(&[(PATH, 5)]);
    let target = packinfo(&[(PATH, 2)]);
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();
    write_live(&instance_root, PATH, b"corrupted-on-disk-bytes").await;
    write_staged(&staging_dir, PATH, b"repaired-pack-bytes").await;

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Present { md5: hashes(9).md5 });

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::Repair {
            re_enable_disabled: false,
        },
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a damaged enabled file with staged bytes repairs without error");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::Replace);
    assert_eq!(
        entries[0].reason,
        PlanReason::RepairOverwrote {
            original: hashes(2).md5,
            current: hashes(9).md5,
        }
    );

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error repairing a damaged enabled file");

    let bytes = tokio::fs::read(live_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(
        bytes, b"repaired-pack-bytes",
        "repair must overwrite corrupted bytes with the pack's own"
    );

    let audit = render_audit(&entries, &[]);
    assert!(
        audit.contains("\nFiles replaced:\n - /mods/damaged.jar\n"),
        "audit was:\n{audit}"
    );
}

#[tokio::test]
async fn repair_missing_enabled_file_is_recreated_from_staged_bytes() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/mods/deleted.jar";

    let old = packinfo(&[(PATH, 5)]);
    let target = packinfo(&[(PATH, 2)]);
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();
    write_staged(&staging_dir, PATH, b"restored-pack-bytes").await;
    // Nothing written live: the user deleted it.

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Missing);

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::Repair {
            re_enable_disabled: false,
        },
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a missing enabled file with staged bytes repairs without error");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::Create);
    assert_eq!(entries[0].reason, PlanReason::RepairRestored);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error restoring a missing enabled file");

    let bytes = tokio::fs::read(live_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(bytes, b"restored-pack-bytes");

    let audit = render_audit(&entries, &[]);
    assert!(
        audit.contains("\nFiles created:\n - /mods/deleted.jar\n"),
        "audit was:\n{audit}"
    );
}

#[tokio::test]
async fn repair_edited_override_is_reset_to_pack_bytes() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/config/user-edited.json";

    let old = packinfo(&[(PATH, 5)]);
    let target = packinfo(&[(PATH, 2)]);
    let staged: HashSet<String> = [PATH.to_string()].into_iter().collect();
    write_live(&instance_root, PATH, b"{\"user\":\"edited this by hand\"}").await;
    write_staged(&staging_dir, PATH, b"{\"pack\":\"default\"}").await;

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Present { md5: hashes(9).md5 });

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::Repair {
            re_enable_disabled: false,
        },
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a hand-edited config with staged bytes repairs without error");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::Replace);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error resetting an edited config");

    let bytes = tokio::fs::read(live_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(
        bytes, b"{\"pack\":\"default\"}",
        "repair must reset a user edit to the pack's own declared bytes, \
         unlike a plain version change which preserves it"
    );
}

#[tokio::test]
async fn repair_saves_folder_execute_plan_leaves_damaged_file_untouched() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const PATH: &str = "/saves/world/level.dat";

    let old = packinfo(&[(PATH, 1)]);
    let target = packinfo(&[(PATH, 2)]);
    // Deliberately nothing staged for it — /saves must never need a staged
    // source to stay untouched, in either mode.
    let staged: HashSet<String> = HashSet::new();
    write_live(&instance_root, PATH, b"the-players-actual-world-bytes").await;

    let mut disk = HashMap::new();
    disk.insert(PATH.to_string(), DiskState::Present { md5: hashes(9).md5 });

    let entries = apply_plan::plan(PlanInputs {
        old: Some(&old),
        target: &target,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::Repair {
            re_enable_disabled: false,
        },
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a save file must never error, even damaged relative to target");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].action, PlanAction::Keep);
    assert_eq!(entries[0].reason, PlanReason::InSaveFolder);

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must not error on a Keep entry");

    let bytes = tokio::fs::read(live_path(&instance_root, PATH))
        .await
        .unwrap();
    assert_eq!(
        bytes, b"the-players-actual-world-bytes",
        "repair must never touch a /saves file, damaged or not"
    );
}

// --- apply_user_cleanup (repair's user-requested
// --- removals) — never bails (a bad entry warns-and-skips, see the
// --- function's own doc comment for why), and every syntax check goes
// --- through normalize_cleanup_path, exhaustively unit-tested on its own
// --- terms in `modpack/mod.rs`. These prove apply_user_cleanup actually
// --- wires that normalizer, the tracked-path (incl. `.disabled`-twin) and
// --- symlink-containment checks, and real removal together correctly.

#[tokio::test]
async fn apply_user_cleanup_removes_untracked_path_and_reports_it() {
    let (_tmp, instance_root, _staging_dir) = scaffold();
    const PATH: &str = "/mods/leftover-experiment.jar";
    write_live(&instance_root, PATH, b"no-longer-wanted").await;

    // A tracked sibling under /mods puts "mods" in the walk's top-level
    // allow-list — an empty packinfo now means an empty allow-list, hence
    // an empty walk, by design (the walk is restricted to directories the
    // pack actually references).
    let target = packinfo(&[("/mods/tracked-sibling.jar", 1)]);
    let removed =
        apply_user_cleanup(&[PATH.to_string()], None, &target, &instance_root, false).await;

    assert_eq!(removed, vec![PATH.to_string()]);
    assert!(
        !live_path(&instance_root, PATH).exists(),
        "the file must actually be removed from disk"
    );
}

#[tokio::test]
async fn apply_user_cleanup_accepts_a_path_whose_filename_merely_contains_a_dotdot_substring() {
    // Contrast with the OLD, buggy `path.contains("..")` substring check
    // this replaced: "foo..bar.jar" is a perfectly legal filename with no
    // ParentDir component anywhere in it, and must be removable.
    let (_tmp, instance_root, _staging_dir) = scaffold();
    const PATH: &str = "/mods/foo..bar.jar";
    write_live(&instance_root, PATH, b"a-legitimately-named-file").await;

    // See apply_user_cleanup_removes_untracked_path_and_reports_it for why
    // a tracked /mods sibling is needed: an empty packinfo now means an
    // empty walk-restriction allow-list.
    let target = packinfo(&[("/mods/tracked-sibling.jar", 1)]);
    let removed =
        apply_user_cleanup(&[PATH.to_string()], None, &target, &instance_root, false).await;

    assert_eq!(removed, vec![PATH.to_string()]);
    assert!(!live_path(&instance_root, PATH).exists());
}

#[tokio::test]
async fn apply_user_cleanup_skips_a_spelling_that_is_not_an_exact_walked_member() {
    // Pins the walk-membership mechanism directly, without needing an
    // actually case-insensitive filesystem to run on: on Linux,
    // "/mods/Extra.jar" and "/mods/extra.jar" are simply two different
    // strings, so if the real on-disk file is spelled "extra.jar", a
    // cleanup request for "Extra.jar" is not an exact member of the walked
    // set and must be skipped — the identical mechanism that structurally
    // closes the case-insensitive-filesystem aliasing class on Windows and
    // default-configuration macOS (GDL's two largest platforms), where the
    // two spellings really would resolve to the same file.
    let (_tmp, instance_root, _staging_dir) = scaffold();
    const REAL: &str = "/mods/extra.jar";
    const REQUESTED: &str = "/mods/Extra.jar";
    write_live(&instance_root, REAL, b"real-file-bytes").await;

    // A tracked /mods sibling — see
    // apply_user_cleanup_removes_untracked_path_and_reports_it for why: an
    // empty packinfo means an empty top-level allow-list, so the walk would
    // never even reach /mods and the exact-spelling guard below would never
    // actually run.
    let target = packinfo(&[("/mods/real.jar", 1)]);
    let removed = apply_user_cleanup(
        &[REQUESTED.to_string()],
        None,
        &target,
        &instance_root,
        false,
    )
    .await;

    assert!(
        removed.is_empty(),
        "a spelling that is not an exact member of the walked set must be skipped, got {removed:?}"
    );
    assert!(
        live_path(&instance_root, REAL).exists(),
        "the real file must survive a differently-spelled cleanup request untouched"
    );
}

#[tokio::test]
async fn apply_user_cleanup_is_noop_for_already_missing_path() {
    let (_tmp, instance_root, _staging_dir) = scaffold();
    const PATH: &str = "/mods/already-gone.jar";
    // Deliberately never written to disk.

    let target = packinfo(&[]);
    let removed =
        apply_user_cleanup(&[PATH.to_string()], None, &target, &instance_root, false).await;

    assert!(
        removed.is_empty(),
        "a path that was never on disk must not be reported as removed"
    );
}

#[tokio::test]
async fn apply_user_cleanup_skips_path_tracked_in_target() {
    let (_tmp, instance_root, _staging_dir) = scaffold();
    const PATH: &str = "/mods/pack-owned.jar";
    write_live(&instance_root, PATH, b"pack-owned-bytes").await;

    let target = packinfo(&[(PATH, 1)]);
    let removed =
        apply_user_cleanup(&[PATH.to_string()], None, &target, &instance_root, false).await;

    assert!(
        removed.is_empty(),
        "cleaning up a path the target pack still ships must be skipped"
    );
    assert!(
        live_path(&instance_root, PATH).exists(),
        "a skipped cleanup must not touch the file"
    );
}

#[tokio::test]
async fn apply_user_cleanup_skips_path_tracked_in_old_only() {
    let (_tmp, instance_root, _staging_dir) = scaffold();
    const PATH: &str = "/mods/dropped-from-target.jar";
    write_live(&instance_root, PATH, b"stale-pack-bytes").await;

    // Present in old (a prior version shipped it) but not target — still
    // pack-tracked history, not the user's own file to clean up.
    let old = packinfo(&[(PATH, 1)]);
    let target = packinfo(&[]);
    let removed = apply_user_cleanup(
        &[PATH.to_string()],
        Some(&old),
        &target,
        &instance_root,
        false,
    )
    .await;

    assert!(
        removed.is_empty(),
        "cleaning up a path only the OLD packinfo tracks must still be skipped"
    );
    assert!(live_path(&instance_root, PATH).exists());
}

#[tokio::test]
async fn apply_user_cleanup_skips_a_tracked_paths_disabled_twin_spelling() {
    // packinfo always keys a disabled pack file under its ENABLED spelling
    // (packinfo::scan_dir strips the suffix) — a literal-string tracked
    // check against a cleanup_paths entry spelled with `.disabled` would
    // otherwise miss this and delete a tracked file's disabled twin.
    let (_tmp, instance_root, _staging_dir) = scaffold();
    const BARE: &str = "/mods/tracked.jar";
    const TWIN_CLEANUP_ENTRY: &str = "/mods/tracked.jar.disabled";
    write_disabled(&instance_root, BARE, b"disabled-twin-bytes").await;

    let target = packinfo(&[(BARE, 1)]);
    let removed = apply_user_cleanup(
        &[TWIN_CLEANUP_ENTRY.to_string()],
        None,
        &target,
        &instance_root,
        false,
    )
    .await;

    assert!(
        removed.is_empty(),
        "cleaning up a tracked path's .disabled twin spelling must be skipped too"
    );
    assert!(
        disabled_path(&instance_root, BARE).exists(),
        "a skipped cleanup must not touch the file"
    );
}

#[tokio::test]
async fn apply_user_cleanup_removes_a_stale_disabled_twin_coexisting_with_its_enabled_copy() {
    // Contrast with apply_user_cleanup_skips_a_tracked_paths_disabled_twin_spelling
    // just above: THAT twin is the tracked path's sole on-disk copy (planner-owned,
    // must survive). THIS twin coexists beside a live enabled copy of the very same
    // tracked path — the stale-leftover scenario — and is pure redundant
    // garbage
    // the planner will never look at again, so cleanup must actually remove it.
    let (_tmp, instance_root, _staging_dir) = scaffold();
    const BARE: &str = "/mods/tracked.jar";
    const TWIN_CLEANUP_ENTRY: &str = "/mods/tracked.jar.disabled";
    write_live(&instance_root, BARE, b"live-enabled-bytes").await;
    write_disabled(&instance_root, BARE, b"stale-twin-bytes").await;

    let target = packinfo(&[(BARE, 1)]);
    let removed = apply_user_cleanup(
        &[TWIN_CLEANUP_ENTRY.to_string()],
        None,
        &target,
        &instance_root,
        false,
    )
    .await;

    assert_eq!(
        removed,
        vec![TWIN_CLEANUP_ENTRY.to_string()],
        "the coexisting stale twin must be reported as removed"
    );
    assert!(
        !disabled_path(&instance_root, BARE).exists(),
        "the stale twin must actually be gone from disk"
    );
    assert!(
        live_path(&instance_root, BARE).exists(),
        "the pack's own live enabled copy must survive untouched"
    );
}

#[tokio::test]
async fn apply_user_cleanup_skips_every_syntactically_invalid_path_without_bailing() {
    let (_tmp, instance_root, _staging_dir) = scaffold();
    // A legitimate tracked mod present too, so "/./mods/tracked.jar" has a
    // real tracked file it could otherwise have deleted if the CurDir
    // component were ever tolerated.
    const TRACKED: &str = "/mods/tracked.jar";
    write_live(&instance_root, TRACKED, b"pack-owned-bytes").await;
    let target = packinfo(&[(TRACKED, 1)]);

    let bad_paths = [
        "/./saves/world/level.dat",
        "//etc/passwd",
        "/",
        "/./mods/tracked.jar",
    ]
    .into_iter()
    .map(String::from)
    .collect::<Vec<_>>();

    let removed = apply_user_cleanup(&bad_paths, None, &target, &instance_root, false).await;

    assert!(
        removed.is_empty(),
        "every syntactically invalid path must be skipped, not removed: {removed:?}"
    );
    assert!(
        live_path(&instance_root, TRACKED).exists(),
        "the tracked file must survive the '/./mods/tracked.jar' entry untouched"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn apply_user_cleanup_refuses_to_follow_a_symlinked_parent_out_of_the_instance() {
    let (_tmp, instance_root, _staging_dir) = scaffold();

    // A directory OUTSIDE the instance root entirely, holding a file that
    // must survive no matter what.
    let outside = tempfile::tempdir().unwrap();
    let victim = outside.path().join("victim.txt");
    tokio::fs::write(&victim, b"must-survive").await.unwrap();

    // "mods/escape" is a symlink pointing at the outside directory, so
    // "/mods/escape/victim.txt" is syntactically pristine (no `.`/`..`
    // component anywhere) yet resolves, once the symlinked parent is
    // actually followed, to a path physically outside the instance data
    // dir — exactly the case component-level normalisation alone cannot
    // catch, which is what the canonicalize+containment check is for.
    let mods_dir = instance_root.join("instance").join("mods");
    tokio::fs::create_dir_all(&mods_dir).await.unwrap();
    std::os::unix::fs::symlink(outside.path(), mods_dir.join("escape")).unwrap();

    // A tracked /mods sibling — see
    // apply_user_cleanup_removes_untracked_path_and_reports_it for why: an
    // empty packinfo means an empty top-level allow-list, so the walk would
    // never even descend into /mods and the containment check below would
    // never actually run.
    let target = packinfo(&[("/mods/real.jar", 1)]);
    let removed = apply_user_cleanup(
        &["/mods/escape/victim.txt".to_string()],
        None,
        &target,
        &instance_root,
        false,
    )
    .await;

    assert!(
        removed.is_empty(),
        "a path resolving outside the instance data dir via a symlinked parent must be skipped"
    );
    assert_eq!(
        tokio::fs::read(&victim).await.unwrap(),
        b"must-survive",
        "the file outside the instance root must be completely untouched"
    );
}

// --- execute_plan containment guard -----------------------------------------

#[tokio::test]
async fn execute_plan_never_escapes_the_instance_root() {
    // Bypasses apply_plan::plan (and so parse_packinfo's own key validation)
    // to hand execute_plan a PlanEntry it never validated itself — proving
    // the belt-and-braces containment check right before `remove_file` is
    // what actually stops the escape, not just upstream validation.
    let (_tmp, instance_root, staging_dir) = scaffold();

    let outside = tempfile::tempdir().unwrap();
    let victim = outside.path().join("victim.txt");
    tokio::fs::write(&victim, b"must-survive").await.unwrap();

    let entries = vec![PlanEntry {
        // A doubled leading slash: stripping one leaves the absolute tail
        // that would otherwise redirect `data_path.join(rel)` at `victim`
        // once combined with the outside path below.
        path: format!("/{}", victim.display()),
        action: PlanAction::Delete,
        reason: PlanReason::PackDropped,
    }];

    let result = execute_plan(&entries, &instance_root, &staging_dir).await;

    assert!(
        result.is_err(),
        "a plan entry escaping the instance root must error, not proceed"
    );
    assert_eq!(
        tokio::fs::read(&victim).await.unwrap(),
        b"must-survive",
        "the file outside the instance root must be completely untouched"
    );
}

#[tokio::test]
async fn execute_plan_rejects_a_dotdot_segment_even_though_starts_with_alone_would_miss_it() {
    // `Path::starts_with` compares path components lexically and never
    // resolves `..`, so the plain `live.starts_with(&data_path)` check
    // alone does not catch this: "mods/../../evil.jar" joined onto the data
    // dir still lexically "starts with" it even though the real,
    // OS-resolved path is the data dir's own parent. Bypasses
    // apply_plan::plan (and so parse_packinfo's own '..'-segment
    // rejection) entirely, proving execute_plan's own re-check — not
    // upstream validation — is what stops this escape.
    let (_tmp, instance_root, staging_dir) = scaffold();
    // `mods` must physically exist under the data dir for the filesystem to
    // walk through it while resolving the `..`s below.
    tokio::fs::create_dir_all(instance_root.join("instance").join("mods"))
        .await
        .unwrap();
    let victim = instance_root.join("evil.jar");
    tokio::fs::write(&victim, b"must-survive").await.unwrap();

    let entries = vec![PlanEntry {
        path: "/mods/../../evil.jar".to_string(),
        action: PlanAction::Delete,
        reason: PlanReason::PackDropped,
    }];

    let result = execute_plan(&entries, &instance_root, &staging_dir).await;

    assert!(
        result.is_err(),
        "a '..' segment must be rejected before ever reaching remove_file"
    );
    assert_eq!(
        tokio::fs::read(&victim).await.unwrap(),
        b"must-survive",
        "a '..'-segment escape must never actually delete anything outside the data dir"
    );
}

// --- (e) audit format golden -------------------------------------------------

/// The format contract `installAudit.ts` parses. Every `PlanAction` and every
/// `PlanReason` appears at least once, paired the way the planner actually
/// pairs them (see `apply_plan.rs`) rather than merely being syntactically
/// valid. Get every byte of the existing sections exactly right —
/// `apps/desktop/e2e-tests/helpers/installAudit.ts` throws on an
/// unrecognised reason rather than silently skipping it. Precedent for
/// goldens: `managers::minecraft::processor_outputs::processor_outputs_golden`.
#[test]
fn render_audit_golden() {
    let entries = vec![
        PlanEntry {
            path: "/mods/pack-update.jar".to_string(),
            action: PlanAction::Replace,
            reason: PlanReason::PackUpdate,
        },
        PlanEntry {
            path: "/mods/unchanged.jar".to_string(),
            action: PlanAction::Keep,
            reason: PlanReason::Unchanged,
        },
        PlanEntry {
            path: "/config/modified-by-user.json".to_string(),
            action: PlanAction::Keep,
            reason: PlanReason::ModifiedByUser {
                original: hashes(1).md5,
                current: hashes(9).md5,
            },
        },
        PlanEntry {
            path: "/mods/deleted-by-user.jar".to_string(),
            action: PlanAction::Keep,
            reason: PlanReason::DeletedByUser,
        },
        PlanEntry {
            path: "/mods/disabled-by-user.jar".to_string(),
            action: PlanAction::Keep,
            reason: PlanReason::DisabledByUser,
        },
        PlanEntry {
            path: "/saves/world/level.dat".to_string(),
            action: PlanAction::Keep,
            reason: PlanReason::InSaveFolder,
        },
        PlanEntry {
            path: "/mods/pack-dropped.jar".to_string(),
            action: PlanAction::Delete,
            reason: PlanReason::PackDropped,
        },
        PlanEntry {
            path: "/mods/disabled-replace-resumed.jar".to_string(),
            action: PlanAction::Delete,
            reason: PlanReason::DisabledReplaceResumed,
        },
        PlanEntry {
            path: "/mods/dropped-but-modified.jar".to_string(),
            action: PlanAction::Keep,
            reason: PlanReason::DroppedButModified {
                original: hashes(3).md5,
                current: hashes(4).md5,
            },
        },
        PlanEntry {
            path: "/config/preserved-existing.json".to_string(),
            action: PlanAction::Keep,
            reason: PlanReason::PreservedExisting,
        },
        PlanEntry {
            path: "/config/case-aliased.json".to_string(),
            action: PlanAction::Keep,
            reason: PlanReason::CaseAliasedByTarget {
                surviving_path: "/config/Case-Aliased.json".to_string(),
            },
        },
        PlanEntry {
            path: "/mods/disabled-repair-overwrote.jar".to_string(),
            action: PlanAction::ReplaceDisabled,
            reason: PlanReason::RepairOverwrote {
                original: hashes(5).md5,
                current: hashes(6).md5,
            },
        },
        PlanEntry {
            path: "/mods/repair-restored.jar".to_string(),
            action: PlanAction::Create,
            reason: PlanReason::RepairRestored,
        },
        PlanEntry {
            path: "/mods/re-enabled.jar".to_string(),
            action: PlanAction::ReEnable,
            reason: PlanReason::ReEnabled,
        },
    ];

    let audit = render_audit(&entries, &["/mods/removed-by-request.jar".to_string()]);

    // Built with push_str, one section line per call, so no source-level
    // line-continuation whitespace-eating can silently swallow a leading
    // " - " space the way a `"...\` multi-line literal would.
    let mut expected = String::new();
    expected.push_str("GDLauncher Modpack Install/Update Audit\n");
    expected.push_str("\nFiles that could not be replaced:\n");
    expected.push_str(" - /config/modified-by-user.json: modified by user\n");
    expected.push_str(&format!(
        "     original md5: {}\n",
        hex::encode(hashes(1).md5)
    ));
    expected.push_str(&format!(
        "     current md5:  {}\n",
        hex::encode(hashes(9).md5)
    ));
    expected.push_str(" - /mods/deleted-by-user.jar: deleted by user\n");
    expected.push_str(" - /mods/disabled-by-user.jar: disabled by user\n");
    expected.push_str(" - /saves/world/level.dat: files in /saves will never be modified\n");
    expected.push_str(" - /mods/dropped-but-modified.jar: modified by user\n");
    expected.push_str(&format!(
        "     original md5: {}\n",
        hex::encode(hashes(3).md5)
    ));
    expected.push_str(&format!(
        "     current md5:  {}\n",
        hex::encode(hashes(4).md5)
    ));
    expected.push_str(" - /config/preserved-existing.json: already present\n");
    expected.push_str(" - /config/case-aliased.json: case-aliased with a tracked path\n");
    expected.push_str("\nFiles deleted:\n");
    expected.push_str(" - /mods/pack-dropped.jar\n");
    expected.push_str(" - /mods/disabled-replace-resumed.jar\n");
    expected.push_str("\nFiles replaced:\n");
    expected.push_str(" - /mods/pack-update.jar\n");
    expected.push_str(" - /mods/disabled-repair-overwrote.jar\n");
    expected.push_str("\nFiles created:\n");
    expected.push_str(" - /mods/repair-restored.jar\n");
    expected.push_str("\nFiles unchanged:\n");
    expected.push_str(" - /mods/unchanged.jar\n");
    expected.push_str("\nFiles re-enabled:\n");
    expected.push_str(" - /mods/re-enabled.jar\n");
    expected.push_str("\nFiles removed at user request:\n");
    expected.push_str(" - /mods/removed-by-request.jar\n");

    assert_eq!(audit, expected);
}

// --- resume after a crash mid-`execute_plan` --------------------------
//
// `process_modpack` only rebuilds `tmp-packinfo.json` by walking
// `.setup/staging` when `.setup/apply-started` is absent (see
// `tmp_packinfo_must_be_regenerated`'s own doc) — once `execute_plan` has
// been called at least once for the current record, that marker stays
// present across every later pass over the same change-pack-version
// session, so `tmp-packinfo.json` is left untouched instead of being
// re-derived. This test proves both halves of why that guard exists: first
// the corruption mechanism it prevents (re-deriving the record from a
// staging dir `execute_plan` has already partially consumed silently drops
// the paths it already moved out), then the fix's actual guarantee (a
// resumed pass reconciling against the untouched, complete pre-crash
// record finishes applying the pending override and never touches the one
// already on disk).

#[tokio::test]
async fn resume_after_partial_apply_preserves_unconsumed_overrides() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    const APPLIED: &str = "/mods/applied.jar";
    const PENDING: &str = "/mods/pending.jar";

    // A fresh modpack install stages two overrides.
    write_staged(&staging_dir, APPLIED, b"applied-bytes").await;
    write_staged(&staging_dir, PENDING, b"pending-bytes").await;

    // The full post-apply record `process_modpack` writes to
    // `tmp-packinfo.json` BEFORE `execute_plan` ever runs, derived (via
    // `packinfo::scan_dir`, the same function `process_modpack` calls) from
    // the fully-populated staging directory.
    let complete_target = super::packinfo::scan_dir(&staging_dir.join("instance"), None)
        .await
        .expect("scanning the fully-populated staging dir must not error");
    assert_eq!(
        complete_target.files.len(),
        2,
        "the pre-apply record must cover both overrides"
    );
    let applied_md5 = complete_target.files.get(APPLIED).unwrap().md5;

    // Round-tripped through `tmp-packinfo.json` on disk exactly like
    // `process_modpack` does, proving the JSON encoding preserves both
    // entries too, not just the in-memory `PackInfo`.
    let tmp_packinfo_path = instance_root.join("tmp-packinfo.json");
    tokio::fs::write(
        &tmp_packinfo_path,
        super::packinfo::make_packinfo(complete_target).unwrap(),
    )
    .await
    .unwrap();

    // Simulate a crash mid-`execute_plan`: APPLIED already got renamed out
    // of staging into the live instance dir (mirroring what the real
    // `PlanAction::Create` arm of `execute_plan` does, `create_dir_all`
    // included); PENDING is still sitting in staging, exactly where
    // extraction left it.
    let applied_live = live_path(&instance_root, APPLIED);
    tokio::fs::create_dir_all(applied_live.parent().unwrap())
        .await
        .unwrap();
    tokio::fs::rename(staged_path(&staging_dir, APPLIED), &applied_live)
        .await
        .unwrap();

    // The actual decision `process_modpack` consults on this exact resumed
    // pass: a modpack file is still on disk (`file.is_some()` would be
    // `true`) and `execute_plan` already started consuming staging before
    // the crash (`apply_started` is `true` — it was written right before
    // the interrupted pass called it, and only `.setup` being wiped removes
    // it). A guard that ignored whether the apply had actually started
    // would regenerate here.
    let target_packinfo = if super::tmp_packinfo_must_be_regenerated(true, true) {
        // What that regeneration would produce if it walked staging again
        // right now: this is the corruption mechanism the guard exists to
        // prevent, exercised here via the very function `process_modpack`
        // itself calls to derive it.
        super::packinfo::scan_dir(&staging_dir.join("instance"), None)
            .await
            .expect("scanning the partially-consumed staging dir must not error")
    } else {
        // The fix: `tmp-packinfo.json` was left untouched across the
        // resume, so this is what `process_modpack_staging` actually loads
        // as the target.
        super::packinfo::parse_packinfo(
            &tokio::fs::read_to_string(&tmp_packinfo_path).await.unwrap(),
        )
        .unwrap()
    };

    assert!(
        target_packinfo.files.contains_key(APPLIED),
        "the surviving packinfo record must still cover the already-applied path — a guard \
         that regenerates on a resumed pass loses it by walking a staging dir that no longer \
         has it"
    );
    assert_eq!(
        target_packinfo.files.len(),
        2,
        "the surviving packinfo record must still cover both paths"
    );

    let mut disk = HashMap::new();
    disk.insert(APPLIED.to_string(), DiskState::Present { md5: applied_md5 });
    // PENDING was never applied, so disk has nothing for it (defaults to
    // Missing in `apply_plan::plan`).

    let staged: HashSet<String> = [PENDING.to_string()].into_iter().collect();

    let entries = apply_plan::plan(PlanInputs {
        old: None,
        target: &target_packinfo,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a resumed pass against the preserved record must not error");

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must finish applying the unconsumed override");

    assert_eq!(
        tokio::fs::read(live_path(&instance_root, APPLIED))
            .await
            .unwrap(),
        b"applied-bytes",
        "the already-applied override must survive the resumed pass untouched"
    );
    assert_eq!(
        tokio::fs::read(live_path(&instance_root, PENDING))
            .await
            .unwrap(),
        b"pending-bytes",
        "the not-yet-applied override must still get created from its staged bytes"
    );
}

// --- crash between promote and staging-removal: idempotent completion -----
//
// `process_modpack_staging` promotes `tmp-packinfo.json` to `packinfo.json`
// BEFORE removing `staging_dir`, not after: once that rename lands,
// `packinfo.json` alone fully describes the target state, so a crash
// landing between the two calls is recoverable by finishing the leftover
// cleanup rather than re-deriving anything. `finish_promoted_staging` is
// exactly that leftover cleanup — this test lands in that exact gap
// (`packinfo.json` already the promoted record, `tmp-packinfo.json` gone,
// `staging_dir` still around) and checks the resumed cleanup completes
// cleanly without ever touching `packinfo.json`.

#[tokio::test]
async fn resume_after_promote_before_staging_removal_completes_cleanly() {
    let (_tmp, instance_root, staging_dir) = scaffold();
    let setup_path = staging_dir.parent().unwrap().to_path_buf();

    // The promote already landed: packinfo.json is the fully-promoted new
    // record; tmp-packinfo.json no longer exists.
    let target = packinfo(&[("/mods/a.jar", 1), ("/mods/b.jar", 2)]);
    let target_json = super::packinfo::make_packinfo(target).unwrap();
    tokio::fs::write(instance_root.join("packinfo.json"), &target_json)
        .await
        .unwrap();
    assert!(!instance_root.join("tmp-packinfo.json").exists());

    // Leftover staged bytes the crash never got to clean up.
    write_staged(&staging_dir, "/mods/a.jar", b"leftover").await;

    finish_promoted_staging(&staging_dir, &setup_path)
        .await
        .expect("resuming into an already-promoted staging pass must complete cleanly");

    assert!(!staging_dir.exists(), "staging must be fully cleaned up");
    assert!(
        setup_path.join("modpack-complete").exists(),
        "the modpack must be recorded complete"
    );
    assert_eq!(
        tokio::fs::read_to_string(instance_root.join("packinfo.json"))
            .await
            .unwrap(),
        target_json,
        "the promoted packinfo.json must be left exactly as it was — it is already the new record"
    );
}

// --- the widest window: overrides extracted, apply not yet started --------
//
// Override extraction finishing (what used to be the sole `skip_overrides`
// signal) happens well before `tmp-packinfo.json` is ever written — the mod
// download phase and the packinfo-generation walk both sit in between. A
// crash anywhere in that gap leaves an extraction-is-done marker set while
// `.setup/staging` is still completely UNCONSUMED (nothing has renamed
// anything out of it yet) and neither `staging-packinfo.json` nor
// `tmp-packinfo.json` exists. Keying the regeneration decision on
// "overrides extracted" alone (rather than "has execute_plan actually
// started") gets this window wrong in two different ways, covered by the
// two tests below.

#[tokio::test]
async fn resume_with_overrides_extracted_but_apply_not_started_regenerates_and_applies_everything()
{
    let (_tmp, instance_root, staging_dir) = scaffold();
    const A: &str = "/mods/a.jar";
    const B: &str = "/config/b.cfg";

    // Extraction already finished for this session — both target files are
    // staged and fully unconsumed, since `execute_plan` has never run yet
    // for this record.
    write_staged(&staging_dir, A, b"a-bytes").await;
    write_staged(&staging_dir, B, b"b-bytes").await;

    let setup_path = staging_dir.parent().unwrap().to_path_buf();
    let staging_packinfo_path = setup_path.join("staging-packinfo.json");
    let tmp_packinfo_path = instance_root.join("tmp-packinfo.json");

    // The exact window: neither record has been written yet.
    assert!(!staging_packinfo_path.exists());
    assert!(!tmp_packinfo_path.exists());

    // Real markers on a real `.setup`, exactly as `process_modpack` would
    // leave them mid-window: extraction already created its own
    // "overrides done" marker (a directory, matching
    // `skip_overrides_path.is_dir()`), but `execute_plan` genuinely never
    // ran, so `.setup/apply-started` was never written.
    let skip_overrides_style_marker = setup_path.join("modpack-skip-overrides");
    tokio::fs::create_dir_all(&skip_overrides_style_marker)
        .await
        .unwrap();
    let apply_started_marker = setup_path.join("apply-started");
    assert!(!apply_started_marker.exists());

    // Two candidate discriminators for the exact same window, read from the
    // exact real files a resumed `process_modpack` would check. Both feed
    // the same `tmp_packinfo_must_be_regenerated` arithmetic (which never
    // changed) — the divergence is entirely in which marker gets read, i.e.
    // in `process_modpack`'s own (App-dependent, not directly unit-testable
    // here) call site. Spelled out explicitly so this test documents,
    // rather than merely asserts, exactly what regressing that call site
    // back to the pre-fix marker would do.
    assert!(
        !super::tmp_packinfo_must_be_regenerated(true, skip_overrides_style_marker.is_dir()),
        "sanity: keying regeneration on \"overrides extracted\" alone (the pre-fix design) \
         would wrongly suppress it here — extraction finished long before the apply itself, or \
         even tmp-packinfo.json, ever existed"
    );
    assert!(
        super::tmp_packinfo_must_be_regenerated(true, apply_started_marker.exists()),
        "the real discriminator (has execute_plan started, read from .setup/apply-started) \
         must regenerate here — staging is fully unconsumed, so re-deriving both records is \
         always safe, and skipping it leaves staging-packinfo.json unwritten, which \
         process_modpack_staging reads as \"nothing staged\" and deletes the whole \
         freshly-downloaded pack without applying anything"
    );

    // Perform the regeneration exactly as `process_modpack` does: scan the
    // fully-populated staging dir and write both records.
    let target = super::packinfo::scan_dir(&staging_dir.join("instance"), None)
        .await
        .expect("scanning the fully-populated staging dir must not error");
    assert_eq!(
        target.files.len(),
        2,
        "regeneration must see every unconsumed override"
    );
    // staging-packinfo.json's content is never read again, only its
    // existence is checked (see process_modpack_staging's early return) —
    // write a placeholder, exactly enough to prove the write itself happens.
    tokio::fs::write(&staging_packinfo_path, "[]")
        .await
        .unwrap();
    tokio::fs::write(
        &tmp_packinfo_path,
        super::packinfo::make_packinfo(target).unwrap(),
    )
    .await
    .unwrap();

    // process_modpack_staging must now see a real record to apply against
    // instead of taking its "nothing staged" early return.
    assert!(staging_packinfo_path.exists());

    let target_packinfo = super::packinfo::parse_packinfo(
        &tokio::fs::read_to_string(&tmp_packinfo_path).await.unwrap(),
    )
    .unwrap();

    let staged: HashSet<String> = [A.to_string(), B.to_string()].into_iter().collect();
    let disk: HashMap<String, DiskState> = HashMap::new(); // both missing on disk so far

    let entries = apply_plan::plan(PlanInputs {
        old: None,
        target: &target_packinfo,
        staged: &staged,
        disk: &disk,
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
        coexisting_disabled_twin_md5: &HashMap::new(),
    })
    .expect("a fully-staged, unconsumed apply must not error");

    execute_plan(&entries, &instance_root, &staging_dir)
        .await
        .expect("execute_plan must apply both files");

    assert_eq!(
        tokio::fs::read(live_path(&instance_root, A)).await.unwrap(),
        b"a-bytes",
        "both overrides must actually land on disk — nothing was silently dropped"
    );
    assert_eq!(
        tokio::fs::read(live_path(&instance_root, B)).await.unwrap(),
        b"b-bytes"
    );
}

#[test]
fn already_promoted_inference_requires_apply_started_not_just_an_old_packinfo() {
    // Genuinely promoted: `execute_plan` ran (marker present) and its
    // record was renamed into place. The only combination that may take
    // the "already applied, just finish cleanup" fast path.
    assert!(super::staging_apply_already_promoted(true, true));

    // Fresh install landing in the overrides-extracted-but-not-yet-applied
    // window: no packinfo has ever existed (nothing installed before), and
    // the apply hasn't started. Must NOT be read as "already promoted" — a
    // guard that did would mark the change complete having applied nothing,
    // permanently (every future launch lands in the same state).
    assert!(!super::staging_apply_already_promoted(false, false));

    // Version change on an already-installed modpack landing in the same
    // window: an OLD packinfo.json genuinely exists (from before this
    // change began), but the apply for *this* change hasn't started. The
    // old packinfo's mere existence must not be mistaken for proof this
    // apply finished — same failure mode as above, just with a
    // pre-existing file this time instead of no file at all. The pre-fix
    // inline condition (`!tmp_packinfo_path.exists()` alone deciding
    // "already promoted" whenever `packinfo_path.exists()`, with no
    // apply-started check at all) reduces to exactly `packinfo_exists` on
    // its own — computed here from the same input this case feeds the real
    // function, so the two are asserted to actually disagree rather than
    // just independently asserted.
    let apply_started = false;
    let packinfo_exists = true;
    let pre_fix_would_treat_as_promoted = packinfo_exists; // the bare, pre-fix condition
    let fixed = super::staging_apply_already_promoted(apply_started, packinfo_exists);
    assert_ne!(
        pre_fix_would_treat_as_promoted, fixed,
        "the fixed discriminator must disagree with the pre-fix bare-packinfo_exists one for \
         exactly this input (old packinfo present, apply not started) — that divergence is the \
         whole point of the fix; the pre-fix condition would have wrongly taken the fast path \
         and marked the change complete having applied nothing"
    );
    assert!(!fixed);

    // Marker present but no packinfo at all: `apply_started` alone isn't
    // sufficient either — a genuine invariant violation, must not be
    // silently treated as promoted.
    assert!(!super::staging_apply_already_promoted(true, false));
}

// --- apply-started marker durability -------------------------------------

#[tokio::test]
async fn fsync_dir_succeeds_on_a_real_directory() {
    // `fsync_dir` is the other half of durably creating
    // `.setup/apply-started` (see the write site in
    // `process_modpack_staging`): a file's own `sync_all` only flushes its
    // bytes, this flushes the directory entry that makes it discoverable.
    // Exercised here against a real directory rather than only compiled —
    // `File::open` on a bare directory (legal on Unix for this purpose,
    // rejected on Windows, where the function is a no-op instead) is easy
    // to get subtly wrong.
    let tmp = tempfile::tempdir().unwrap();
    super::fsync_dir(tmp.path())
        .await
        .expect("fsyncing a real, existing directory must not error");
}

/// A VirtualBox shared folder answers every folder sync with EINVAL and some
/// network mounts with ENOTSUP: `fsync_dir` leaves those entries to the
/// system, so a setup (a game version changed in Settings among them) still
/// runs where 2.0.40 ran it. A real error still fails.
#[cfg(unix)]
#[tokio::test]
async fn fsync_dir_is_done_where_the_volume_does_not_sync_folders() {
    use crate::util::durable::test_hooks::FolderSyncs;
    let tmp = tempfile::tempdir().unwrap();
    for refusal in [libc::EINVAL, libc::ENOTSUP] {
        let syncs = FolderSyncs::failing_with(tmp.path(), refusal);
        super::fsync_dir(tmp.path()).await.unwrap();
        assert_eq!(syncs.synced(), [tmp.path()], "errno {refusal}");
    }
    let _syncs = FolderSyncs::failing_with(tmp.path(), libc::EIO);
    assert!(super::fsync_dir(tmp.path()).await.is_err());
}

#[tokio::test]
async fn apply_started_marker_is_durably_creatable_and_discoverable() {
    // The exact sequence `process_modpack_staging` runs right before
    // `execute_plan`: create the marker file, fsync it, fsync its parent
    // directory. Proves the sequence itself completes and leaves the marker
    // present and readable — not a substitute for a real power-loss test
    // (not feasible here), but does prove `fsync_dir` and `File::create` +
    // `sync_all` compose without erroring against a real `.setup`-shaped
    // directory, and that the marker is exactly what `apply_started_path.exists()`
    // (the read side, in both `process_modpack` and `process_modpack_staging`)
    // expects to find.
    let tmp = tempfile::tempdir().unwrap();
    let setup_path = tmp.path().join(".setup");
    tokio::fs::create_dir_all(&setup_path).await.unwrap();
    let marker_path = setup_path.join("apply-started");

    assert!(!marker_path.exists());

    {
        let marker_file = tokio::fs::File::create(&marker_path).await.unwrap();
        marker_file.sync_all().await.unwrap();
    }
    super::fsync_dir(&setup_path).await.unwrap();

    assert!(
        marker_path.exists(),
        "the marker must be discoverable immediately after the sync sequence completes"
    );
}

#[tokio::test]
async fn task03_uppercase_save_is_preserved_by_real_execution() {
    let (_tmp, root, staging) = scaffold();
    let path = "/Saves/world/level.dat";
    write_live(&root, path, b"played world").await;
    write_staged(&staging, path, b"bundled world").await;
    let target = packinfo(&[(path, 2)]);
    let universe = [path.to_owned()].into_iter().collect();
    let scan = disk_scan::scan_disk_state(&root.join("instance"), &universe)
        .await
        .unwrap();
    let entries = apply_plan::plan(PlanInputs {
        old: Some(&target),
        target: &target,
        staged: &[path.to_owned()].into_iter().collect(),
        disk: &scan.states,
        coexisting_disabled_twin_md5: &scan.coexisting_disabled_twin_md5,
        mode: ApplyMode::Repair {
            re_enable_disabled: false,
        },
        fs_case_insensitive: true,
    })
    .unwrap();
    execute_plan(&entries, &root, &staging).await.unwrap();
    assert_eq!(
        std::fs::read(live_path(&root, path)).unwrap(),
        b"played world"
    );
}

#[tokio::test]
async fn task03_late_save_create_is_preserved() {
    let (_tmp, root, staging) = scaffold();
    let path = "/saves/world/level.dat";
    write_staged(&staging, path, b"bundled world").await;
    let target = packinfo(&[(path, 2)]);
    let entries = apply_plan::plan(PlanInputs {
        old: None,
        target: &target,
        staged: &[path.to_owned()].into_iter().collect(),
        disk: &HashMap::new(),
        coexisting_disabled_twin_md5: &HashMap::new(),
        mode: ApplyMode::Repair {
            re_enable_disabled: false,
        },
        fs_case_insensitive: false,
    })
    .unwrap();
    write_live(&root, path, b"late played world").await;
    execute_plan(&entries, &root, &staging).await.unwrap();
    assert_eq!(
        std::fs::read(live_path(&root, path)).unwrap(),
        b"late played world"
    );
    assert_eq!(
        std::fs::read(staged_path(&staging, path)).unwrap(),
        b"bundled world"
    );
}

#[tokio::test]
async fn task03_disabled_replace_failure_retains_source_for_retry() {
    let (_tmp, root, staging) = scaffold();
    let path = "/mods/b.jar";
    write_staged(&staging, "/mods/b.jar.disabled", b"target disabled").await;
    // Reject a non-file opposite before publication; retained staged input
    // remains retryable once the invalid directory is repaired. The separate
    // chmod regressions cover real IO failure after publication.
    std::fs::create_dir_all(live_path(&root, path)).unwrap();
    let entries = vec![PlanEntry {
        path: path.into(),
        action: PlanAction::Replace,
        reason: PlanReason::PackUpdate,
    }];
    assert!(execute_plan(&entries, &root, &staging).await.is_err());
    assert_eq!(
        std::fs::read(staged_path(&staging, "/mods/b.jar.disabled")).unwrap(),
        b"target disabled"
    );
    assert!(!disabled_path(&root, path).exists());
    std::fs::remove_dir(live_path(&root, path)).unwrap();
    write_live(&root, path, b"old bare").await;
    execute_plan(&entries, &root, &staging).await.unwrap();
    assert!(!live_path(&root, path).exists());
    assert_eq!(
        std::fs::read(disabled_path(&root, path)).unwrap(),
        b"target disabled"
    );
}

#[tokio::test]
async fn task03_apply_refreshes_skip_oracle() {
    let (_tmp, root, staging) = scaffold();
    let path = "/mods/foo.jar";
    let (_, oracle) = disk_scan::scan_instance_as_packinfo(&root.join("instance"))
        .await
        .unwrap();
    write_disabled(&root, path, b"disabled mod").await;
    write_staged(&staging, path, b"disabled mod").await;
    let universe = [path.to_owned()].into_iter().collect();
    let scan = super::scan_for_apply(&root.join("instance"), &universe, Some(oracle))
        .await
        .unwrap();
    assert!(matches!(
        scan.states.get(path),
        Some(DiskState::Disabled { .. })
    ));
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let entries = apply_plan::plan(PlanInputs {
        old: Some(&target),
        target: &target,
        staged: &[path.to_owned()].into_iter().collect(),
        disk: &scan.states,
        coexisting_disabled_twin_md5: &scan.coexisting_disabled_twin_md5,
        mode: ApplyMode::Repair {
            re_enable_disabled: false,
        },
        fs_case_insensitive: false,
    })
    .unwrap();
    let outcomes = super::execute_plan(&entries, &root, &staging, &target)
        .await
        .unwrap();
    assert!(!live_path(&root, path).exists());
    assert_eq!(
        std::fs::read(disabled_path(&root, path)).unwrap(),
        b"disabled mod"
    );
    assert!(render_audit(&outcomes, &[]).contains("/mods/foo.jar: disabled by user"));
}

#[tokio::test]
async fn task03_legacy_cleanup_has_no_retroactive_consent() {
    let (_tmp, root, _staging) = scaffold();
    write_live(&root, "/config/retired.cfg", b"edited settings").await;
    let target = packinfo(&[("/config/current.cfg", 1)]);
    let removed = bound_cleanup(
        &super::RepairMarkerFile {
            cleanup_paths: vec!["/config/retired.cfg".into()],
            ..Default::default()
        },
        None,
        &target,
        &root,
        false,
    )
    .await
    .removed;
    assert!(removed.is_empty());
    assert_eq!(
        std::fs::read(live_path(&root, "/config/retired.cfg")).unwrap(),
        b"edited settings"
    );
}

async fn cleanup_options(root: &std::path::Path, path: &str) -> super::RepairMarkerFile {
    use crate::managers::instance::modpack::{
        CleanupSelection, PackProofContext, create_cleanup_batch, origin_check::hash_file_sha512,
    };
    super::RepairMarkerFile {
        cleanup_selections: vec![CleanupSelection {
            path: path.into(),
            sha512: hex::encode(hash_file_sha512(&live_path(root, path)).await.unwrap()),
            context: PackProofContext {
                platform: "modrinth".into(),
                project_id: "pack".into(),
                version_id: "installed".into(),
            },
            proven: false,
        }],
        cleanup_batch: Some(create_cleanup_batch(root).unwrap()),
        ..Default::default()
    }
}

/// Once the setup is done with them (see `session::dispose`), the copies its
/// apply moved aside go, as 2.0.40 deleted them during its apply: a file the
/// user ticked for removal, and the damaged twin a re-enable replaced from
/// its staged copy. A ticked file that changed while it was moved is put
/// back, and its copy stays.
#[tokio::test]
async fn settled_recovery_copies_go_once_the_setup_is_done_with_them() {
    let (_tmp, root, staging) = scaffold();
    const REENABLE: &str = "/mods/reenabled.jar";
    write_disabled(&root, REENABLE, b"damaged").await;
    write_staged(&staging, REENABLE, b"pack bytes").await;
    let entries = vec![PlanEntry {
        path: REENABLE.into(),
        action: PlanAction::ReEnable,
        reason: PlanReason::ReEnabled,
    }];
    execute_plan(&entries, &root, &staging).await.unwrap();
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(super::transition_path(&root, REENABLE)).unwrap())
            .unwrap();
    let twin_batch = root
        .join(".repair-recovery")
        .join(record["recovery_batch"].as_str().unwrap());
    let twin_copy = twin_batch.join(super::record_key(REENABLE)).join("payload");
    assert_eq!(std::fs::read(&twin_copy).unwrap(), b"damaged");

    let (ticked, changed) = ("/config/ticked.cfg", "/config/changed.cfg");
    write_live(&root, ticked, b"ticked").await;
    write_live(&root, changed, b"as ticked").await;
    let mut options = cleanup_options(&root, ticked).await;
    let unused = cleanup_options(&root, changed).await;
    options.cleanup_selections.extend(unused.cleanup_selections);
    let changed_live = live_path(&root, changed);
    let edit = changed_live.clone();
    hook("cleanup-move", changed_live.clone(), move || {
        std::fs::write(edit, b"edited meanwhile").unwrap()
    });
    std::fs::write(
        root.join(".setup/repair"),
        serde_json::to_vec(&options).unwrap(),
    )
    .unwrap();
    let report = bound_cleanup(
        &options,
        None,
        &packinfo(&[("/config/current.cfg", 1)]),
        &root,
        false,
    )
    .await;
    assert_eq!(report.removed, [ticked], "{:?}", report.notes);
    let batch = root
        .join(".repair-recovery")
        .join(options.cleanup_batch.as_ref().unwrap());
    assert!(
        batch
            .join(super::record_key(ticked))
            .join("payload")
            .is_file()
    );
    let kept = batch.join(super::record_key(changed)).join("payload");
    assert_eq!(std::fs::read(&kept).unwrap(), b"edited meanwhile");

    super::prune_settled_recovery(&root, &root.join(".setup")).await;
    assert!(!twin_batch.exists());
    assert!(!batch.join(super::record_key(ticked)).exists());
    assert_eq!(std::fs::read(&kept).unwrap(), b"edited meanwhile");
    assert_eq!(std::fs::read(&changed_live).unwrap(), b"edited meanwhile");
    assert!(!live_path(&root, ticked).exists());
    // The batch the second selection made and nothing used.
    assert!(
        !root
            .join(".repair-recovery")
            .join(unused.cleanup_batch.as_ref().unwrap())
            .exists()
    );
    assert_eq!(
        std::fs::read(live_path(&root, REENABLE)).unwrap(),
        b"pack bytes"
    );
}

#[tokio::test]
async fn task03_selected_cleanup_preserves_later_edit() {
    let (_tmp, root, _) = scaffold();
    let path = "/config/retired.cfg";
    write_live(&root, path, b"selected bytes").await;
    let options = cleanup_options(&root, path).await;
    write_live(&root, path, b"new edits").await;
    let target = packinfo(&[("/config/current.cfg", 1)]);
    let removed = bound_cleanup(&options, None, &target, &root, false)
        .await
        .removed;
    assert!(removed.is_empty());
    assert_eq!(std::fs::read(live_path(&root, path)).unwrap(), b"new edits");
}

#[tokio::test]
async fn task03_cleanup_retains_payload_after_finalization_and_retry() {
    let (_tmp, root, staging) = scaffold();
    let path = "/config/retired.cfg";
    write_live(&root, path, b"selected bytes").await;
    let options = cleanup_options(&root, path).await;
    let target = packinfo(&[("/config/current.cfg", 1)]);
    assert_eq!(
        bound_cleanup(&options, None, &target, &root, false)
            .await
            .removed,
        vec![path]
    );
    let recovery = root
        .join(".repair-recovery")
        .join(options.cleanup_batch.as_ref().unwrap());
    let entry = std::fs::read_dir(&recovery)
        .unwrap()
        .next()
        .expect("retained entry")
        .unwrap()
        .path();
    assert_eq!(
        std::fs::read(entry.join("payload")).unwrap(),
        b"selected bytes"
    );
    assert!(entry.join("entry.json").exists());
    write_live(&root, path, b"replacement after cleanup").await;
    bound_cleanup(&options, None, &target, &root, false).await;
    assert_eq!(
        std::fs::read(live_path(&root, path)).unwrap(),
        b"replacement after cleanup"
    );
    finish_promoted_staging(&staging, &root.join(".setup"))
        .await
        .unwrap();
    finish_promoted_staging(&staging, &root.join(".setup"))
        .await
        .unwrap();
    std::fs::create_dir_all(root.join(".install_audit")).unwrap();
    std::fs::write(root.join(".install_audit/audit.txt"), b"later audit").unwrap();
    std::fs::remove_dir_all(root.join(".install_audit")).unwrap();
    std::fs::remove_dir_all(root.join(".setup")).unwrap();
    std::fs::create_dir(root.join(".setup")).unwrap();
    assert!(entry.join("entry.json").exists());
    assert_eq!(
        std::fs::read(entry.join("payload")).unwrap(),
        b"selected bytes"
    );
}

// Path-scoped one-shot interleavings; only tests install hooks. The actual
// publication/removal/move operations remain the production filesystem IO.
type Hook = Box<dyn FnOnce() + Send>;
static HOOKS: std::sync::LazyLock<std::sync::Mutex<HashMap<(&'static str, PathBuf), Hook>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));
pub(in crate::managers::instance) fn hook(
    event: &'static str,
    path: PathBuf,
    callback: impl FnOnce() + Send + 'static,
) {
    HOOKS
        .lock()
        .unwrap()
        .insert((event, path), Box::new(callback));
}
pub(super) fn run_hook(event: &'static str, path: &std::path::Path) {
    let callback = HOOKS.lock().unwrap().remove(&(event, path.to_owned()));
    if let Some(callback) = callback {
        callback();
    }
}

/// A save written after the plan, just before the staged world is linked
/// into place, is kept: the link never replaces what arrived first.
#[tokio::test]
async fn task03_no_clobber_at_actual_publication_boundary_and_audit() {
    let (_tmp, root, staging) = scaffold();
    let path = "/saves/new/level.dat";
    write_staged(&staging, path, b"complete target").await;
    let destination = live_path(&root, path);
    let late = destination.clone();
    hook("publish", destination, move || {
        std::fs::write(late, b"late user world").unwrap()
    });
    let entries = vec![PlanEntry {
        path: path.into(),
        action: PlanAction::Create,
        reason: PlanReason::RepairRestored,
    }];
    let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();
    assert_eq!(
        std::fs::read(live_path(&root, path)).unwrap(),
        b"late user world"
    );
    assert_eq!(
        std::fs::read(staged_path(&staging, path)).unwrap(),
        b"complete target"
    );
    assert!(!same_file::is_same_file(live_path(&root, path), staged_path(&staging, path)).unwrap());
    assert_eq!(outcomes[0].action, PlanAction::Keep);
    assert_eq!(outcomes[0].reason, PlanReason::PreservedExisting);
    assert!(render_audit(&outcomes, &[]).contains("/saves/new/level.dat: already present"));
}

#[cfg(unix)]
#[tokio::test]
async fn task03_permission_failure_resumes_original_disabled_repair_options() {
    use std::os::unix::fs::PermissionsExt;
    for flag in [false, true] {
        let (_tmp, root, staging) = scaffold();
        let path = "/mods/b.jar";
        write_live(&root, path, b"old bare").await;
        write_staged(&staging, "/mods/b.jar.disabled", b"verified target").await;
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let marker = super::RepairMarkerFile {
            re_enable_disabled: flag,
            ..Default::default()
        };
        let marker_bytes = serde_json::to_vec(&marker).unwrap();
        let target_bytes = super::packinfo::make_packinfo(PackInfo {
            files: target.files.clone(),
        })
        .unwrap()
        .into_bytes();
        std::fs::write(root.join(".setup/repair"), &marker_bytes).unwrap();
        std::fs::write(root.join("tmp-packinfo.json"), &target_bytes).unwrap();
        let live = live_path(&root, path);
        let parent = live.parent().unwrap().to_owned();
        let locked = parent.clone();
        hook("remove-bare", live, move || {
            std::fs::set_permissions(locked, std::fs::Permissions::from_mode(0o555)).unwrap()
        });
        let universe = [path.to_owned()].into_iter().collect();
        for attempt in 0..2 {
            let recorded: super::RepairMarkerFile =
                serde_json::from_slice(&std::fs::read(root.join(".setup/repair")).unwrap())
                    .unwrap();
            let scan = super::scan_for_apply(&root.join("instance"), &universe, None)
                .await
                .unwrap();
            let entries = apply_plan::plan(PlanInputs {
                old: Some(&target),
                target: &target,
                staged: &[path.to_owned()].into_iter().collect(),
                disk: &scan.states,
                coexisting_disabled_twin_md5: &scan.coexisting_disabled_twin_md5,
                mode: ApplyMode::Repair {
                    re_enable_disabled: recorded.re_enable_disabled,
                },
                fs_case_insensitive: false,
            })
            .unwrap();
            let result = super::execute_plan(&entries, &root, &staging, &target).await;
            std::fs::set_permissions(&parent, std::fs::Permissions::from_mode(0o755)).unwrap();
            if attempt == 0 {
                assert!(result.is_err(), "real permission-denied removal");
                assert_eq!(std::fs::read(live_path(&root, path)).unwrap(), b"old bare");
            } else {
                result.unwrap();
                assert!(!live_path(&root, path).exists());
            }
            assert_eq!(
                std::fs::read(staged_path(&staging, "/mods/b.jar.disabled")).unwrap(),
                b"verified target"
            );
            assert_eq!(
                std::fs::read(disabled_path(&root, path)).unwrap(),
                b"verified target"
            );
            assert_eq!(
                std::fs::read(root.join(".setup/repair")).unwrap(),
                marker_bytes
            );
            assert_eq!(
                std::fs::read(root.join("tmp-packinfo.json")).unwrap(),
                target_bytes
            );
        }
    }
}

#[tokio::test]
async fn task03_cleanup_move_race_retains_and_restores_without_clobber() {
    for occupied in [false, true] {
        let (_tmp, root, _) = scaffold();
        let path = "/config/retired.cfg";
        write_live(&root, path, b"selected").await;
        let options = cleanup_options(&root, path).await;
        let live = live_path(&root, path);
        let edit = live.clone();
        hook("cleanup-move", live.clone(), move || {
            std::fs::write(edit, b"racing edits").unwrap()
        });
        if occupied {
            let new_live = live.clone();
            hook("cleanup-moved", live, move || {
                std::fs::write(new_live, b"new occupant").unwrap()
            });
        }
        let result = bound_cleanup(
            &options,
            None,
            &packinfo(&[("/config/current.cfg", 1)]),
            &root,
            false,
        )
        .await;
        assert!(result.removed.is_empty());
        assert_eq!(
            std::fs::read(live_path(&root, path)).unwrap(),
            if occupied {
                b"new occupant".as_slice()
            } else {
                b"racing edits".as_slice()
            }
        );
        let batch = root
            .join(".repair-recovery")
            .join(options.cleanup_batch.as_ref().unwrap());
        let entry = std::fs::read_dir(batch)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        assert_eq!(
            std::fs::read(entry.join("payload")).unwrap(),
            b"racing edits"
        );
        assert!(
            result
                .notes
                .join("\n")
                .contains(&entry.join("payload").display().to_string())
        );
    }
}

#[tokio::test]
async fn task03_interrupted_cleanup_finishes_same_payload_without_taking_new_file() {
    let (_tmp, root, _) = scaffold();
    let path = "/config/retired.cfg";
    write_live(&root, path, b"selected").await;
    let options = cleanup_options(&root, path).await;
    let entry = root
        .join(".repair-recovery")
        .join(options.cleanup_batch.as_ref().unwrap())
        .join(super::record_key(path));
    std::fs::create_dir(&entry).unwrap();
    let record = super::CleanupRecoveryEntry {
        schema_version: 1,
        selection: options.cleanup_selections[0].clone(),
        payload: "payload".into(),
        status: "pending".into(),
    };
    super::write_cleanup_record(&entry, &record).await.unwrap();
    std::fs::rename(live_path(&root, path), entry.join("payload")).unwrap();
    std::fs::write(entry.join("payload"), b"changed during interruption").unwrap();
    let report = bound_cleanup(
        &options,
        None,
        &packinfo(&[("/config/current.cfg", 1)]),
        &root,
        false,
    )
    .await;
    assert!(report.removed.is_empty());
    assert_eq!(
        std::fs::read(live_path(&root, path)).unwrap(),
        b"changed during interruption"
    );
    assert_eq!(
        std::fs::read(entry.join("payload")).unwrap(),
        b"changed during interruption"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn task03_physical_save_alias_and_sensitive_distinct_name() {
    use std::os::unix::fs::symlink;
    let (_tmp, root, staging) = scaffold();
    // /Saves is a folder of its own on a case-sensitive volume, and the saves
    // folder itself on a case-insensitive one (default APFS on macOS).
    let insensitive = disk_scan::probe_case_insensitive(&root.join("instance")).await;
    write_live(&root, "/saves/world/level.dat", b"played").await;
    symlink("saves", root.join("instance/world-alias")).unwrap();
    write_staged(&staging, "/world-alias/world/level.dat", b"bundled").await;
    if !insensitive {
        write_live(
            &root,
            "/Saves/world/level.dat",
            b"separate sensitive content",
        )
        .await;
    }
    write_staged(&staging, "/Saves/world/level.dat", b"ordinary update").await;
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let scan = super::scan_for_apply(
        &root.join("instance"),
        &target.files.keys().cloned().collect(),
        None,
    )
    .await
    .unwrap();
    let mut entries = apply_plan::plan(PlanInputs {
        old: Some(&target),
        target: &target,
        staged: &target.files.keys().cloned().collect(),
        disk: &scan.states,
        coexisting_disabled_twin_md5: &scan.coexisting_disabled_twin_md5,
        mode: ApplyMode::Repair {
            re_enable_disabled: false,
        },
        fs_case_insensitive: insensitive,
    })
    .unwrap();
    disk_scan::protect_resolved_saves(&mut entries, &root.join("instance"), Some(&target));
    let outcomes = super::execute_plan(&entries, &root, &staging, &target)
        .await
        .unwrap();
    assert_eq!(
        std::fs::read(live_path(&root, "/saves/world/level.dat")).unwrap(),
        b"played"
    );
    // A distinct /Saves takes the pack's update; the saves folder spelled
    // /Saves keeps the played world.
    assert_eq!(
        std::fs::read(live_path(&root, "/Saves/world/level.dat")).unwrap(),
        if insensitive {
            b"played".as_slice()
        } else {
            b"ordinary update".as_slice()
        }
    );
    assert!(render_audit(&outcomes, &[]).contains("/world-alias/world/level.dat: files in /saves"));
    let candidates =
        super::walk_untracked_files(&root.join("instance"), None, &target, insensitive).await;
    assert!(!candidates.contains_key("/world-alias/world/level.dat"));
}

#[cfg(unix)]
#[tokio::test]
async fn task03_case_ownership_fresh_scan_actual_update_and_modified_control() {
    use std::os::unix::fs::symlink;
    for modified in [false, true] {
        let (_tmp, root, staging) = scaffold();
        write_live(&root, "/Config/test.cfg", b"old").await;
        let old = super::packinfo::scan_dir(&root.join("instance"), None)
            .await
            .unwrap();
        // A case-insensitive volume (default APFS on macOS) already resolves
        // config to Config; a case-sensitive one gets the same alias as a link.
        if !disk_scan::probe_case_insensitive(&root.join("instance")).await {
            symlink("Config", root.join("instance/config")).unwrap();
        }
        if modified {
            write_live(&root, "/Config/test.cfg", b"user edits").await;
        }
        write_staged(&staging, "/config/test.cfg", b"new").await;
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let universe = old
            .files
            .keys()
            .chain(target.files.keys())
            .cloned()
            .collect();
        let scan = super::scan_for_apply(&root.join("instance"), &universe, None)
            .await
            .unwrap();
        let entries = apply_plan::plan(PlanInputs {
            old: Some(&old),
            target: &target,
            staged: &target.files.keys().cloned().collect(),
            disk: &scan.states,
            coexisting_disabled_twin_md5: &scan.coexisting_disabled_twin_md5,
            mode: ApplyMode::VersionChange,
            fs_case_insensitive: true,
        })
        .unwrap();
        let outcomes = super::execute_plan(&entries, &root, &staging, &target)
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(live_path(&root, "/Config/test.cfg")).unwrap(),
            if modified {
                b"user edits".as_slice()
            } else {
                b"new".as_slice()
            }
        );
        assert_eq!(
            outcomes
                .iter()
                .filter(|e| e.action == PlanAction::Replace)
                .count(),
            usize::from(!modified)
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn task03_publication_io_error_and_hash_mismatch_retain_source() {
    use std::os::unix::fs::PermissionsExt;
    let (_tmp, root, staging) = scaffold();
    let path = "/mods/new.jar";
    write_staged(&staging, path, b"complete target").await;
    std::fs::create_dir_all(root.join("instance/mods")).unwrap();
    let source = staged_path(&staging, path);
    let destination = live_path(&root, path);
    assert!(
        super::publish_staged_copy(&source, &destination, Some([0; 64]), true)
            .await
            .is_err()
    );
    assert!(!destination.exists());
    let parent = destination.parent().unwrap().to_owned();
    let locked = parent.clone();
    hook("publish", destination.clone(), move || {
        std::fs::set_permissions(locked, std::fs::Permissions::from_mode(0o555)).unwrap()
    });
    let expected: [u8; 64] = sha2::Sha512::digest(b"complete target").into();
    let result = super::publish_staged_copy(&source, &destination, Some(expected), true).await;
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err());
    assert!(!destination.exists());
    assert_eq!(std::fs::read(&source).unwrap(), b"complete target");
    assert!(
        super::publish_staged_copy(&source, &destination, Some(expected), true)
            .await
            .unwrap()
    );
    assert_eq!(std::fs::read(destination).unwrap(), b"complete target");
    assert!(source.exists());
}

#[tokio::test]
async fn task03_cleanup_context_change_preserves_selected_file() {
    let (_tmp, root, _) = scaffold();
    let path = "/config/retired.cfg";
    write_live(&root, path, b"selected").await;
    let options = cleanup_options(&root, path).await;
    let mut context = options.cleanup_selections[0].context.clone();
    context.version_id = "changed".into();
    let report = super::apply_bound_cleanup(
        &options,
        None,
        &packinfo(&[("/config/current.cfg", 1)]),
        &root,
        false,
        Some(&context),
        None,
    )
    .await;
    assert!(report.removed.is_empty());
    assert_eq!(std::fs::read(live_path(&root, path)).unwrap(), b"selected");
}

#[test]
fn task03_cleanup_audit_preserves_title_and_list_contract() {
    let entries = vec![PlanEntry {
        path: "/mods/new.jar".into(),
        action: PlanAction::Create,
        reason: PlanReason::PackUpdate,
    }];
    let cleanup = super::CleanupReport {
        removed: vec!["/config/old.cfg".into()],
        notes: vec!["Cleanup recovery: .repair-recovery/cleanup-example/".into()],
    };
    let audit = super::render_cleanup_audit(&entries, &cleanup);
    assert!(audit.starts_with("GDLauncher Modpack Install/Update Audit\nCleanup recovery:"));
    assert!(audit.contains("Files created:\n - /mods/new.jar\n"));
    assert!(audit.contains(" - /config/old.cfg\n"));
    assert_eq!(
        super::render_cleanup_audit(&entries, &super::CleanupReport::default()),
        render_audit(&entries, &[])
    );
}

static IO_FAULTS: std::sync::LazyLock<std::sync::Mutex<HashSet<(&'static str, PathBuf)>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashSet::new()));
pub(in crate::managers::instance) fn fail_io(event: &'static str, path: PathBuf) {
    IO_FAULTS.lock().unwrap().insert((event, path));
}
pub(super) fn io_fault(event: &'static str, path: &std::path::Path) -> std::io::Result<()> {
    if IO_FAULTS.lock().unwrap().remove(&(event, path.to_owned())) {
        return Err(std::io::Error::other("one-shot durability barrier failure"));
    }
    Ok(())
}

/// One-shot hard-link failures, keyed by the publication's final destination
/// (a replacement links to a private sibling first).
static LINK_FAULTS: std::sync::LazyLock<std::sync::Mutex<HashMap<PathBuf, std::io::ErrorKind>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));
fn fail_link(destination: PathBuf, kind: std::io::ErrorKind) {
    LINK_FAULTS.lock().unwrap().insert(destination, kind);
}
pub(super) fn link_fault(destination: &std::path::Path) -> Option<std::io::Error> {
    count_attempt("link", destination);
    if under_any(&NO_LINKS, destination) {
        // EPERM on Linux vfat and exfat; ERROR_INVALID_FUNCTION, the same
        // code, from Windows FAT.
        return Some(std::io::Error::from_raw_os_error(1));
    }
    LINK_FAULTS
        .lock()
        .unwrap()
        .remove(destination)
        .map(std::io::Error::from)
}

/// Folders on a volume that refuses every hard link (FAT32, exFAT, some
/// network shares), and folders that no move from staging reaches (another
/// drive), for every publication under them. Each test names its own.
static NO_LINKS: std::sync::LazyLock<std::sync::Mutex<Vec<PathBuf>>> =
    std::sync::LazyLock::new(Default::default);
static OTHER_DRIVE: std::sync::LazyLock<std::sync::Mutex<Vec<PathBuf>>> =
    std::sync::LazyLock::new(Default::default);
pub(in crate::managers::instance) fn without_hard_links(dir: PathBuf) {
    NO_LINKS.lock().unwrap().push(dir);
}
pub(in crate::managers::instance) fn on_another_drive(dir: PathBuf) {
    OTHER_DRIVE.lock().unwrap().push(dir);
}
fn under_any(
    dirs: &std::sync::LazyLock<std::sync::Mutex<Vec<PathBuf>>>,
    path: &std::path::Path,
) -> bool {
    dirs.lock().unwrap().iter().any(|dir| path.starts_with(dir))
}
pub(super) fn move_fault(destination: &std::path::Path) -> Option<std::io::Error> {
    count_attempt("move", destination);
    under_any(&OTHER_DRIVE, destination)
        .then(|| std::io::Error::from_raw_os_error(crate::util::publish::fault::CROSS_DEVICE))
}

/// Link and move attempts per destination, so parallel tests count only
/// their own.
static ATTEMPTS: std::sync::LazyLock<std::sync::Mutex<HashMap<(&'static str, PathBuf), usize>>> =
    std::sync::LazyLock::new(Default::default);
fn count_attempt(step: &'static str, destination: &std::path::Path) {
    *ATTEMPTS
        .lock()
        .unwrap()
        .entry((step, destination.to_owned()))
        .or_default() += 1;
}
/// How many `step` ("link" or "move") attempts were made under `dir`.
pub(in crate::managers::instance) fn attempts_under(
    step: &'static str,
    dir: &std::path::Path,
) -> usize {
    ATTEMPTS
        .lock()
        .unwrap()
        .iter()
        .filter(|((made, path), _)| *made == step && path.starts_with(dir))
        .map(|(_, count)| count)
        .sum()
}

/// Directory syncs per path, so parallel tests count only their own.
/// Windows has no directory sync, so nothing is recorded there.
static SYNCS: std::sync::LazyLock<std::sync::Mutex<HashMap<PathBuf, usize>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(HashMap::new()));
#[cfg_attr(windows, allow(dead_code))]
pub(super) fn record_sync(dir: &std::path::Path) {
    *SYNCS.lock().unwrap().entry(dir.to_owned()).or_default() += 1;
}
pub(in crate::managers::instance) fn syncs_under(dir: &std::path::Path) -> HashMap<PathBuf, usize> {
    SYNCS
        .lock()
        .unwrap()
        .iter()
        .filter(|(path, _)| path.starts_with(dir))
        .map(|(path, count)| (path.clone(), *count))
        .collect()
}

#[cfg(unix)]
#[tokio::test]
async fn task03_r1_recovery_ancestor_barriers_precede_move() {
    let (_tmp, root, _) = scaffold();
    let path = "/config/retired.cfg";
    write_live(&root, path, b"selected").await;
    let options = cleanup_options(&root, path).await;
    let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for (name, directory) in [
        ("root", root.clone()),
        ("recovery", root.join(".repair-recovery")),
        (
            "batch",
            root.join(".repair-recovery")
                .join(options.cleanup_batch.as_ref().unwrap()),
        ),
    ] {
        let seen = seen.clone();
        hook("sync-directory", directory, move || {
            seen.lock().unwrap().push(name)
        });
    }
    hook("cleanup-move", live_path(&root, path), move || {
        assert_eq!(*seen.lock().unwrap(), vec!["root", "recovery", "batch"])
    });
    let report = bound_cleanup(
        &options,
        None,
        &packinfo(&[("/config/current.cfg", 1)]),
        &root,
        false,
    )
    .await;
    assert_eq!(report.removed, [path]);
}

#[cfg(unix)]
#[tokio::test]
async fn task03_r1_recovery_ancestor_failure_preserves_live() {
    for ancestor in ["root", "recovery"] {
        let (_tmp, root, _) = scaffold();
        let path = "/config/retired.cfg";
        write_live(&root, path, b"selected").await;
        let options = cleanup_options(&root, path).await;
        fail_io(
            "sync-directory",
            if ancestor == "root" {
                root.clone()
            } else {
                root.join(".repair-recovery")
            },
        );
        let report = bound_cleanup(
            &options,
            None,
            &packinfo(&[("/config/current.cfg", 1)]),
            &root,
            false,
        )
        .await;
        assert!(report.removed.is_empty());
        assert_eq!(std::fs::read(live_path(&root, path)).unwrap(), b"selected");
    }
}

#[cfg(unix)]
#[tokio::test]
async fn task03_r1_payload_sync_failure_is_not_reported_as_retained_success() {
    let (_tmp, root, _) = scaffold();
    let path = "/config/retired.cfg";
    write_live(&root, path, b"selected").await;
    let options = cleanup_options(&root, path).await;
    let payload = root
        .join(".repair-recovery")
        .join(options.cleanup_batch.as_ref().unwrap())
        .join(super::record_key(path))
        .join("payload");
    fail_io("sync-file", payload.clone());
    let report = bound_cleanup(
        &options,
        None,
        &packinfo(&[("/config/current.cfg", 1)]),
        &root,
        false,
    )
    .await;
    assert!(report.removed.is_empty());
    assert_eq!(std::fs::read(payload).unwrap(), b"selected");
    assert!(
        report
            .notes
            .join("\n")
            .contains("durability barrier failure")
    );
}

async fn round1_apply(
    root: &std::path::Path,
    staging: &std::path::Path,
    target: &PackInfo,
    options: &super::RepairMarkerFile,
) -> anyhow::Result<String> {
    let universe = target.files.keys().cloned().collect();
    let scan = super::scan_for_apply(&root.join("instance"), &universe, None).await?;
    let staged = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await?
        .files
        .into_keys()
        .collect();
    let entries = super::plan_staging(
        PlanInputs {
            old: Some(target),
            target,
            staged: &staged,
            disk: &scan.states,
            coexisting_disabled_twin_md5: &scan.coexisting_disabled_twin_md5,
            mode: ApplyMode::Repair {
                re_enable_disabled: options.re_enable_disabled,
            },
            fs_case_insensitive: false,
        },
        root,
        Some(options),
        None,
    )
    .await?;
    let mut report = super::CleanupReport::default();
    let actual = super::execute_bound_plan(
        &entries,
        root,
        staging,
        target,
        Some(options),
        &mut report.notes,
        None,
        &Default::default(),
    )
    .await?;
    Ok(super::render_cleanup_audit(&actual, &report))
}

#[cfg(unix)]
#[tokio::test]
async fn task03_r1_reenable_removal_interruption_resumes_same_files_and_audit() {
    use std::os::unix::fs::PermissionsExt;
    let mut audits = Vec::new();
    for interrupted in [false, true] {
        let (_tmp, root, staging) = scaffold();
        let path = "/mods/a.jar";
        write_live(&root, "/mods/a.jar.disabled", b"old twin").await;
        write_staged(&staging, path, b"target").await;
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let options = super::RepairMarkerFile {
            re_enable_disabled: true,
            ..Default::default()
        };
        if interrupted {
            let parent = disabled_path(&root, path).parent().unwrap().to_owned();
            let locked = parent.clone();
            hook("remove-twin", disabled_path(&root, path), move || {
                std::fs::set_permissions(locked, std::fs::Permissions::from_mode(0o555)).unwrap()
            });
            let first = round1_apply(&root, &staging, &target, &options).await;
            std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(first.is_err(), "real first-attempt IO error");
        }
        audits.push(
            round1_apply(&root, &staging, &target, &options)
                .await
                .unwrap(),
        );
        assert_eq!(std::fs::read(live_path(&root, path)).unwrap(), b"target");
        assert!(
            !disabled_path(&root, path).exists(),
            "resume must finish original ReEnable intent"
        );
    }
    assert_eq!(audits[0], audits[1]);
}

#[tokio::test]
async fn task03_r1_completed_disabled_replace_retries_both_flags_same_audit() {
    for flag in [false, true] {
        let (_tmp, root, staging) = scaffold();
        let path = "/mods/a.jar";
        write_live(&root, path, b"old bare").await;
        write_staged(&staging, "/mods/a.jar.disabled", b"target").await;
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let options = super::RepairMarkerFile {
            re_enable_disabled: flag,
            ..Default::default()
        };
        let uninterrupted = round1_apply(&root, &staging, &target, &options)
            .await
            .unwrap();
        let resumed = round1_apply(&root, &staging, &target, &options)
            .await
            .unwrap();
        assert!(
            !live_path(&root, path).exists(),
            "flag {flag} must retain disabled-default Replace outcome"
        );
        assert_eq!(
            std::fs::read(disabled_path(&root, path)).unwrap(),
            b"target"
        );
        assert_eq!(
            uninterrupted, resumed,
            "flag {flag}: audit must describe original completed operation"
        );
    }
}

#[tokio::test]
async fn task03_r1_opposite_edited_during_publication_is_preserved_with_audit() {
    for disabled in [false, true] {
        let (_tmp, root, staging) = scaffold();
        let path = "/mods/a.jar";
        let opposite = if disabled {
            live_path(&root, path)
        } else {
            disabled_path(&root, path)
        };
        let dest = if disabled {
            disabled_path(&root, path)
        } else {
            live_path(&root, path)
        };
        std::fs::create_dir_all(opposite.parent().unwrap()).unwrap();
        std::fs::write(&opposite, b"old").unwrap();
        write_staged(
            &staging,
            if disabled {
                "/mods/a.jar.disabled"
            } else {
                path
            },
            b"target",
        )
        .await;
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let edited = opposite.clone();
        hook("publish", dest, move || {
            std::fs::write(edited, b"external edit").unwrap()
        });
        let options = super::RepairMarkerFile {
            re_enable_disabled: true,
            ..Default::default()
        };
        let audit = round1_apply(&root, &staging, &target, &options)
            .await
            .unwrap();
        assert_eq!(std::fs::read(&opposite).unwrap(), b"external edit");
        assert!(
            audit.contains("Representation conflict preserved"),
            "{audit}"
        );
        assert_eq!(
            round1_apply(&root, &staging, &target, &options)
                .await
                .unwrap(),
            audit
        );
    }
}

#[tokio::test]
async fn task03_r1_transition_binding_rejects_changed_options_target_and_malformed_record() {
    for changed in ["options", "target", "malformed", "foreign"] {
        let (_tmp, root, staging) = scaffold();
        let path = "/mods/a.jar";
        write_live(&root, path, b"old").await;
        write_staged(&staging, "/mods/a.jar.disabled", b"target").await;
        let mut target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let mut options = super::RepairMarkerFile::default();
        round1_apply(&root, &staging, &target, &options)
            .await
            .unwrap();
        let file = super::transition_path(&root, path);
        match changed {
            "options" => options.re_enable_disabled = true,
            "target" => {
                target.files.insert("/mods/another.jar".into(), hashes(42));
            }
            "malformed" => std::fs::write(&file, b"malformed record retained").unwrap(),
            "foreign" => {
                let mut r = super::read_transition(&file).await.unwrap();
                r.schema_version = 999;
                super::write_transition(&file, &r, false).await.unwrap();
            }
            _ => unreachable!(),
        }
        let diagnostic = std::fs::read(&file).unwrap();
        assert!(
            round1_apply(&root, &staging, &target, &options)
                .await
                .is_err(),
            "{changed}"
        );
        assert_eq!(std::fs::read(&file).unwrap(), diagnostic);
        assert!(!live_path(&root, path).exists());
        assert_eq!(
            std::fs::read(disabled_path(&root, path)).unwrap(),
            b"target"
        );
    }
}

#[tokio::test]
async fn task03_r1_completion_record_failure_reuses_original_payload_and_audit() {
    for disabled in [false, true] {
        let mut audits = Vec::new();
        for interrupted in [false, true] {
            let (_tmp, root, staging) = scaffold();
            let path = "/mods/a.jar";
            write_live(
                &root,
                if disabled {
                    path
                } else {
                    "/mods/a.jar.disabled"
                },
                b"original",
            )
            .await;
            write_staged(
                &staging,
                if disabled {
                    "/mods/a.jar.disabled"
                } else {
                    path
                },
                b"target",
            )
            .await;
            let target = super::packinfo::scan_dir(&staging.join("instance"), None)
                .await
                .unwrap();
            let options = super::RepairMarkerFile {
                re_enable_disabled: true,
                ..Default::default()
            };
            if interrupted {
                fail_io("transition-complete", super::transition_path(&root, path));
                assert!(
                    round1_apply(&root, &staging, &target, &options)
                        .await
                        .is_err()
                );
            }
            audits.push(
                round1_apply(&root, &staging, &target, &options)
                    .await
                    .unwrap(),
            );
            let record = super::read_transition(&super::transition_path(&root, path))
                .await
                .unwrap();
            let payload = super::transition_batch(&root, &record)
                .join(super::record_key(path))
                .join("payload");
            assert_eq!(std::fs::read(&payload).unwrap(), b"original");
            assert_eq!(
                std::fs::read_dir(root.join(".repair-recovery"))
                    .unwrap()
                    .count(),
                1
            );
            let opposite = if disabled {
                live_path(&root, path)
            } else {
                disabled_path(&root, path)
            };
            std::fs::write(&opposite, b"new occupant after completion").unwrap();
            let audit = round1_apply(&root, &staging, &target, &options)
                .await
                .unwrap();
            assert!(audit.contains("Representation conflict preserved"));
            assert_eq!(
                std::fs::read(&opposite).unwrap(),
                b"new occupant after completion"
            );
            assert_eq!(std::fs::read(payload).unwrap(), b"original");
        }
        assert_eq!(audits[0], audits[1]);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn task03_r1_intent_and_manifest_write_failure_preserve_live_and_resume() {
    for barrier in ["intent", "manifest"] {
        let (_tmp, root, staging) = scaffold();
        let path = "/mods/a.jar";
        write_live(&root, path, b"original").await;
        write_staged(&staging, "/mods/a.jar.disabled", b"target").await;
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let options = super::RepairMarkerFile::default();
        let record_path = super::transition_path(&root, path);
        if barrier == "intent" {
            fail_io("transition-record", record_path.clone());
        } else {
            // Fail the hierarchy barrier once, leaving only durable intent;
            // then fail its immutable recovery manifest on the next attempt.
            fail_io("sync-directory", root.clone());
            assert!(
                round1_apply(&root, &staging, &target, &options)
                    .await
                    .is_err()
            );
            let record = super::read_transition(&record_path).await.unwrap();
            let manifest = super::transition_batch(&root, &record)
                .join(super::record_key(path))
                .join("entry.json");
            fail_io("transition-record", manifest);
        }
        assert!(
            round1_apply(&root, &staging, &target, &options)
                .await
                .is_err()
        );
        assert_eq!(std::fs::read(live_path(&root, path)).unwrap(), b"original");
        assert!(!disabled_path(&root, path).exists());
        round1_apply(&root, &staging, &target, &options)
            .await
            .unwrap();
        let record = super::read_transition(&record_path).await.unwrap();
        let payload = super::transition_batch(&root, &record)
            .join(super::record_key(path))
            .join("payload");
        assert_eq!(std::fs::read(payload).unwrap(), b"original");
        assert!(!live_path(&root, path).exists());
    }
}

#[cfg(unix)]
#[tokio::test]
async fn task03_r1_conflict_restore_failure_resumes_retained_payload() {
    use std::os::unix::fs::PermissionsExt;
    let (_tmp, root, staging) = scaffold();
    let path = "/mods/a.jar";
    write_live(&root, path, b"original").await;
    write_staged(&staging, "/mods/a.jar.disabled", b"target").await;
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let options = super::RepairMarkerFile::default();
    let opposite = live_path(&root, path);
    let edited = opposite.clone();
    hook("publish", disabled_path(&root, path), move || {
        std::fs::write(edited, b"external edit").unwrap()
    });
    let parent = opposite.parent().unwrap().to_owned();
    let locked = parent.clone();
    hook("publish", opposite.clone(), move || {
        std::fs::set_permissions(locked, std::fs::Permissions::from_mode(0o555)).unwrap()
    });
    let result = round1_apply(&root, &staging, &target, &options).await;
    std::fs::set_permissions(parent, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(result.is_err(), "real denied no-clobber restore");
    let record_path = super::transition_path(&root, path);
    let record = super::read_transition(&record_path).await.unwrap();
    let payload = super::transition_batch(&root, &record)
        .join(super::record_key(path))
        .join("payload");
    assert_eq!(std::fs::read(&payload).unwrap(), b"external edit");
    let audit = round1_apply(&root, &staging, &target, &options)
        .await
        .unwrap();
    assert!(audit.contains("Representation conflict preserved"));
    assert_eq!(std::fs::read(&opposite).unwrap(), b"external edit");
    assert_eq!(std::fs::read(&payload).unwrap(), b"external edit");
    assert_eq!(
        std::fs::read_dir(root.join(".repair-recovery"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        round1_apply(&root, &staging, &target, &options)
            .await
            .unwrap(),
        audit
    );
}

#[tokio::test]
async fn task03_r1_fresh_setup_uses_new_recovery_storage() {
    let (_tmp, root, staging) = scaffold();
    let path = "/mods/a.jar";
    let options = super::RepairMarkerFile::default();
    for original in [b"first original".as_slice(), b"second original".as_slice()] {
        write_live(&root, path, original).await;
        write_staged(&staging, "/mods/a.jar.disabled", b"target").await;
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        round1_apply(&root, &staging, &target, &options)
            .await
            .unwrap();
        let record = super::read_transition(&super::transition_path(&root, path))
            .await
            .unwrap();
        let payload = super::transition_batch(&root, &record)
            .join(super::record_key(path))
            .join("payload");
        assert_eq!(std::fs::read(&payload).unwrap(), original);
        std::fs::remove_dir_all(root.join(".setup")).unwrap();
        std::fs::create_dir_all(staging.join("instance")).unwrap();
        assert_eq!(std::fs::read(&payload).unwrap(), original);
    }
    assert_eq!(
        std::fs::read_dir(root.join(".repair-recovery"))
            .unwrap()
            .count(),
        2
    );
}

#[tokio::test]
async fn task03_r1_pristine_no_download_reenable_has_same_resume_audit() {
    let (_tmp, root, staging) = scaffold();
    let path = "/mods/a.jar";
    write_live(&root, "/mods/a.jar.disabled", b"verified installed bytes").await;
    let target = super::packinfo::scan_dir(&root.join("instance"), None)
        .await
        .unwrap();
    let options = super::RepairMarkerFile {
        re_enable_disabled: true,
        ..Default::default()
    };
    let first = round1_apply(&root, &staging, &target, &options)
        .await
        .unwrap();
    assert_eq!(
        round1_apply(&root, &staging, &target, &options)
            .await
            .unwrap(),
        first
    );
    assert_eq!(
        std::fs::read(live_path(&root, path)).unwrap(),
        b"verified installed bytes"
    );
    assert!(!disabled_path(&root, path).exists());
}

#[cfg(unix)]
#[tokio::test]
async fn task03_r1_cleanup_payload_failure_resume_syncs_both_rename_directories() {
    let (_tmp, root, _staging) = scaffold();
    let path = "/config/retired.cfg";
    write_live(&root, path, b"selected bytes").await;
    let options = cleanup_options(&root, path).await;
    let entry = root
        .join(".repair-recovery")
        .join(options.cleanup_batch.as_ref().unwrap())
        .join(super::record_key(path));
    fail_io("sync-file", entry.join("payload"));
    let target = packinfo(&[("/config/current.cfg", 1)]);
    assert!(
        bound_cleanup(&options, None, &target, &root, false)
            .await
            .removed
            .is_empty()
    );
    assert_eq!(
        std::fs::read(entry.join("payload")).unwrap(),
        b"selected bytes"
    );
    let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    for (name, path) in [
        ("entry", entry.clone()),
        (
            "source",
            live_path(&root, path).parent().unwrap().to_owned(),
        ),
    ] {
        let events = events.clone();
        hook("sync-directory", path, move || {
            events.lock().unwrap().push(name)
        });
    }
    assert_eq!(
        bound_cleanup(&options, None, &target, &root, false)
            .await
            .removed,
        vec![path]
    );
    assert_eq!(*events.lock().unwrap(), vec!["entry", "source"]);
    assert_eq!(
        std::fs::read(entry.join("payload")).unwrap(),
        b"selected bytes"
    );
}

#[tokio::test]
async fn task03_r1_resume_keeps_full_case_ownership_for_other_entries() {
    let (_tmp, root, staging) = scaffold();
    let path = "/mods/a.jar";
    let alias = "/mods/A.jar";
    write_live(&root, "/mods/a.jar.disabled", b"target").await;
    let target = super::packinfo::scan_dir(&root.join("instance"), None)
        .await
        .unwrap();
    let options = super::RepairMarkerFile {
        re_enable_disabled: true,
        ..Default::default()
    };
    round1_apply(&root, &staging, &target, &options)
        .await
        .unwrap();
    let old = PackInfo {
        files: [(alias.into(), target.files[path].clone())]
            .into_iter()
            .collect(),
    };
    let disk = [
        (
            path.into(),
            DiskState::Present {
                md5: target.files[path].md5,
            },
        ),
        (
            alias.into(),
            DiskState::Present {
                md5: target.files[path].md5,
            },
        ),
    ]
    .into_iter()
    .collect();
    for insensitive in [false, true] {
        let entries = super::plan_staging(
            PlanInputs {
                old: Some(&old),
                target: &target,
                staged: &HashSet::new(),
                disk: &disk,
                coexisting_disabled_twin_md5: &HashMap::new(),
                mode: ApplyMode::Repair {
                    re_enable_disabled: true,
                },
                fs_case_insensitive: insensitive,
            },
            &root,
            Some(&options),
            None,
        )
        .await
        .unwrap();
        let alias_entry = entries.iter().find(|e| e.path == alias).unwrap();
        assert_eq!(
            alias_entry.action,
            if insensitive {
                PlanAction::Keep
            } else {
                PlanAction::Delete
            }
        );
        if insensitive {
            assert_eq!(
                alias_entry.reason,
                PlanReason::CaseAliasedByTarget {
                    surviving_path: path.into()
                }
            );
        }
    }
}

/// Like [`scaffold`], under an instance root of 210+ characters built from
/// several components, so it is valid on every OS. Windows (no longPathAware
/// manifest in the test binary or the core) refuses plain Win32 paths of 260+
/// characters, which every record and deep pack file below crosses.
fn long_scaffold() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let mut root = tmp.path().to_path_buf();
    while root.as_os_str().len() < 210 {
        root.push("Vault Hunters 3rd Edition - instance");
    }
    let staging = root.join(".setup").join("staging");
    std::fs::create_dir_all(root.join("instance")).unwrap();
    std::fs::create_dir_all(staging.join("instance")).unwrap();
    (tmp, root, staging)
}

#[tokio::test]
async fn long_instance_root_publishes_deep_files_transitions_and_cleanup() {
    let (_tmp, root, staging) = long_scaffold();
    let deep = format!("/kubejs/{}/{}", "a".repeat(40), "b".repeat(40));
    let create = format!("{deep}/created.json");
    let replace = format!("{deep}/replaced.json");
    const REENABLE: &str = "/mods/pristine.jar";
    let retired = format!("{deep}/retired.cfg");
    assert!(live_path(&root, &create).as_os_str().len() >= 270);

    // (a) Ordinary Create and Replace whose live paths pass 270 characters.
    write_staged(&staging, &create, b"created").await;
    write_staged(&staging, &replace, b"new pack bytes").await;
    write_live(&root, &replace, b"old pack bytes").await;
    // (b) A ReEnable representation transition: intent record and manifest.
    write_disabled(&root, REENABLE, b"pristine").await;
    let entries = vec![
        PlanEntry {
            path: create.clone(),
            action: PlanAction::Create,
            reason: PlanReason::PackUpdate,
        },
        PlanEntry {
            path: REENABLE.into(),
            action: PlanAction::ReEnable,
            reason: PlanReason::ReEnabled,
        },
        PlanEntry {
            path: replace.clone(),
            action: PlanAction::Replace,
            reason: PlanReason::PackUpdate,
        },
    ];
    let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();
    assert_eq!(
        outcomes.iter().map(|e| &e.action).collect::<Vec<_>>(),
        [
            &PlanAction::Create,
            &PlanAction::ReEnable,
            &PlanAction::Replace
        ]
    );
    assert_eq!(
        std::fs::read(live_path(&root, &create)).unwrap(),
        b"created"
    );
    assert_eq!(
        std::fs::read(live_path(&root, &replace)).unwrap(),
        b"new pack bytes"
    );
    assert_eq!(
        std::fs::read(live_path(&root, REENABLE)).unwrap(),
        b"pristine"
    );
    assert!(!disabled_path(&root, REENABLE).exists());
    let record = super::transition_path(&root, REENABLE);
    assert!(record.as_os_str().len() >= 260);
    let transition: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&record).unwrap()).unwrap();
    assert_eq!(transition["completed"], true);
    let recovery = root
        .join(".repair-recovery")
        .join(transition["recovery_batch"].as_str().unwrap())
        .join(super::record_key(REENABLE));
    assert!(recovery.join("entry.json").is_file());
    // Nothing was staged for it, so the twin itself was renamed.
    assert!(!recovery.join("payload").exists());

    // (c) A ticked stale file is moved into retained recovery, not kept.
    write_live(&root, &retired, b"stale").await;
    let options = cleanup_options(&root, &retired).await;
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let report = bound_cleanup(&options, None, &target, &root, false).await;
    assert_eq!(report.removed, [retired.clone()], "{:?}", report.notes);
    assert!(!live_path(&root, &retired).exists());
    let entry = root
        .join(".repair-recovery")
        .join(options.cleanup_batch.as_ref().unwrap())
        .join(super::record_key(&retired));
    assert!(entry.join("entry.json").is_file());
    assert_eq!(std::fs::read(entry.join("payload")).unwrap(), b"stale");
}

#[test]
fn record_names_stay_short_and_distinct() {
    let key = super::record_key("/mods/a.jar");
    assert_eq!(key.len(), 32);
    assert!(key.bytes().all(|b| b.is_ascii_hexdigit()));
    assert_ne!(key, super::record_key("/mods/b.jar"));
    let root = std::path::Path::new("root");
    let manifest = root
        .join(".repair-recovery")
        .join("cleanup-AbC123")
        .join(&key)
        .join("entry.json");
    assert_eq!(manifest.as_os_str().len() - root.as_os_str().len(), 76);
    let record = super::transition_path(root, "/mods/a.jar");
    assert_eq!(record.as_os_str().len() - root.as_os_str().len(), 72);
}

/// macOS volumes without a no-replace rename (exFAT, FAT, NTFS, SMB, NFS)
/// return ENOTSUP from it. Creates still publish there and never overwrite.
#[cfg(unix)]
#[tokio::test]
async fn unsupported_no_replace_rename_still_publishes_without_overwriting() {
    use crate::util::publish::fault::{ENOTSUP, Step, inject, pending};
    let (_tmp, root, staging) = scaffold();
    let path = "/mods/new.jar";
    write_staged(&staging, path, b"complete target").await;
    std::fs::create_dir_all(root.join("instance/mods")).unwrap();
    let source = staged_path(&staging, path);
    let destination = live_path(&root, path);
    let expected: [u8; 64] = sha2::Sha512::digest(b"complete target").into();

    inject(Step::NoReplace, &destination, ENOTSUP);
    assert!(
        super::publish_staged_copy(&source, &destination, Some(expected), true)
            .await
            .unwrap()
    );
    assert!(!pending(Step::NoReplace, &destination));
    assert_eq!(std::fs::read(&destination).unwrap(), b"complete target");

    std::fs::write(&destination, b"user copy").unwrap();
    inject(Step::NoReplace, &destination, ENOTSUP);
    assert!(
        !super::publish_staged_copy(&source, &destination, Some(expected), true)
            .await
            .unwrap()
    );
    assert!(!pending(Step::NoReplace, &destination));
    assert_eq!(std::fs::read(&destination).unwrap(), b"user copy");
    assert_eq!(std::fs::read(&source).unwrap(), b"complete target");
    let names: Vec<_> = std::fs::read_dir(root.join("instance/mods"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name())
        .collect();
    assert_eq!(names, [std::ffi::OsString::from("new.jar")]);
}

/// A mods or config folder junctioned to another drive than the instance
/// root: the representation payload and a ticked cleanup file cross volumes
/// on their way into `.repair-recovery`. They are copied there and the live
/// side removed, so the repair finishes instead of staying in Applying.
#[tokio::test]
async fn cross_volume_recovery_payloads_are_copied_into_recovery() {
    use crate::util::publish::fault::{CROSS_DEVICE, Step, inject, pending};
    let (_tmp, root, staging) = scaffold();
    const REENABLE: &str = "/mods/pristine.jar";
    write_disabled(&root, REENABLE, b"pristine").await;
    // Published from its staged copy, so the twin moves into recovery.
    write_staged(&staging, REENABLE, b"pristine").await;
    let transition = super::transition_path(&root, REENABLE);
    let payload_of = {
        let (root, transition) = (root.clone(), transition.clone());
        move || {
            let record: serde_json::Value =
                serde_json::from_slice(&std::fs::read(&transition).unwrap()).unwrap();
            root.join(".repair-recovery")
                .join(record["recovery_batch"].as_str().unwrap())
                .join(super::record_key(REENABLE))
                .join("payload")
        }
    };
    let arm = payload_of.clone();
    hook("remove-twin", disabled_path(&root, REENABLE), move || {
        inject(Step::NoReplace, &arm(), CROSS_DEVICE)
    });
    let entries = vec![PlanEntry {
        path: REENABLE.into(),
        action: PlanAction::ReEnable,
        reason: PlanReason::ReEnabled,
    }];
    let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();
    assert_eq!(outcomes[0].action, PlanAction::ReEnable);
    let payload = payload_of();
    assert!(
        !pending(Step::NoReplace, &payload),
        "the payload move took the cross-volume copy"
    );
    assert_eq!(
        std::fs::read(live_path(&root, REENABLE)).unwrap(),
        b"pristine"
    );
    assert!(!disabled_path(&root, REENABLE).exists());
    assert_eq!(std::fs::read(&payload).unwrap(), b"pristine");
    let record: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&transition).unwrap()).unwrap();
    assert_eq!(record["completed"], true);
    assert_eq!(record["conflict"], false);

    let retired = "/config/retired.cfg";
    write_live(&root, retired, b"stale").await;
    let options = cleanup_options(&root, retired).await;
    let payload = root
        .join(".repair-recovery")
        .join(options.cleanup_batch.as_ref().unwrap())
        .join(super::record_key(retired))
        .join("payload");
    inject(Step::NoReplace, &payload, CROSS_DEVICE);
    let report = bound_cleanup(
        &options,
        None,
        &packinfo(&[("/config/current.cfg", 1)]),
        &root,
        false,
    )
    .await;
    assert!(!pending(Step::NoReplace, &payload));
    assert_eq!(report.removed, [retired], "{:?}", report.notes);
    assert!(!live_path(&root, retired).exists());
    assert_eq!(std::fs::read(&payload).unwrap(), b"stale");
}

/// Records a pack setup owned by instance 7 whose complete target is the
/// staged tree, the way preparation binds it. Returns the owner.
async fn bind_setup(root: &std::path::Path, staging: &std::path::Path) -> super::Owner {
    let config = crate::managers::instance::schema::parse_instance_config(
        r#"{"_version":"1","name":"Synthetic","game_configuration":{"version":{"release":"1.20.1","modloaders":[]}}}"#,
    )
    .unwrap();
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let mut session =
        super::Session::new(&config, super::Producer::Change, super::Kind::Pack, None).unwrap();
    session.instance_id = Some(7);
    session
        .bind_target(root, staging, &target, Vec::new())
        .await
        .unwrap();
    session.create(root).await.unwrap();
    session.owner().unwrap()
}

/// Replaces the record the way another setup would: a new file renamed over
/// the old one. Its modification time is moved 2 s past the old record's, so
/// the stamp changes even where it has no file identity (Windows) and both
/// writes land in one timestamp tick at the same length.
fn write_record(root: &std::path::Path, session: &super::Session) {
    let record = root.join(".setup").join(super::session::RECORD);
    let before = std::fs::metadata(&record).unwrap().modified().unwrap();
    let private = root.join(".setup").join("record.tmp");
    std::fs::write(&private, serde_json::to_vec_pretty(session).unwrap()).unwrap();
    std::fs::OpenOptions::new()
        .write(true)
        .open(&private)
        .unwrap()
        .set_modified(before + std::time::Duration::from_secs(2))
        .unwrap();
    std::fs::rename(private, record).unwrap();
}

fn foreign(session: &super::Session) -> super::Session {
    let mut foreign = session.clone();
    foreign.operation = uuid::Uuid::new_v4().to_string();
    foreign
}

/// An update over `n` files: a quarter each Keep, Create, Replace and Delete,
/// plus a pack-shipped-disabled Replace and a repair ReEnable.
async fn mixed_update(
    root: &std::path::Path,
    staging: &std::path::Path,
    n: usize,
) -> Vec<PlanEntry> {
    let mut entries = Vec::new();
    for i in 0..n {
        let path = format!("/config/f{i:04}.txt");
        let (action, reason) = match i % 4 {
            0 => {
                write_live(root, &path, b"same").await;
                write_staged(staging, &path, b"same").await;
                (PlanAction::Keep, PlanReason::Unchanged)
            }
            1 => {
                write_staged(staging, &path, b"created").await;
                (PlanAction::Create, PlanReason::PackUpdate)
            }
            2 => {
                write_live(root, &path, b"old").await;
                write_staged(staging, &path, b"new").await;
                (PlanAction::Replace, PlanReason::PackUpdate)
            }
            _ => {
                write_live(root, &path, b"dropped").await;
                (PlanAction::Delete, PlanReason::PackDropped)
            }
        };
        entries.push(PlanEntry {
            path,
            action,
            reason,
        });
    }
    write_live(root, "/mods/shipped.jar", b"old enabled").await;
    write_staged(staging, "/mods/shipped.jar.disabled", b"new disabled").await;
    write_disabled(root, "/mods/reenabled.jar", b"damaged").await;
    write_staged(staging, "/mods/reenabled.jar", b"pack bytes").await;
    entries.push(PlanEntry {
        path: "/mods/reenabled.jar".into(),
        action: PlanAction::ReEnable,
        reason: PlanReason::ReEnabled,
    });
    entries.push(PlanEntry {
        path: "/mods/shipped.jar".into(),
        action: PlanAction::Replace,
        reason: PlanReason::PackUpdate,
    });
    entries.sort_by(|a, b| a.path.cmp(&b.path));
    entries
}

/// Large packs stalled on "Applying staged patches": the whole setup record
/// (target, binding hash, sources) was re-read and re-validated before every
/// file, so the apply was quadratic in the pack size.
#[tokio::test]
async fn apply_checks_setup_ownership_once_whatever_the_pack_size() {
    let mut loads = Vec::new();
    for n in [50, 500] {
        let (_tmp, root, staging) = scaffold();
        let entries = mixed_update(&root, &staging, n).await;
        let owner = bind_setup(&root, &staging).await;
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let before = super::session::load_count::of(&root);
        let mut notes = Vec::new();
        let outcomes = super::execute_bound_plan(
            &entries,
            &root,
            &staging,
            &target,
            None,
            &mut notes,
            Some(&owner),
            &Default::default(),
        )
        .await
        .unwrap();
        loads.push(super::session::load_count::of(&root) - before);

        assert_eq!(outcomes, entries, "{notes:?}");
        for entry in &entries {
            let live = live_path(&root, &entry.path);
            match entry.action {
                PlanAction::Keep => assert_eq!(std::fs::read(&live).unwrap(), b"same"),
                PlanAction::Create => assert_eq!(std::fs::read(&live).unwrap(), b"created"),
                PlanAction::Delete => assert!(!live.exists()),
                PlanAction::Replace if entry.path == "/mods/shipped.jar" => {
                    assert!(!live.exists());
                    assert_eq!(
                        std::fs::read(disabled_path(&root, &entry.path)).unwrap(),
                        b"new disabled"
                    );
                }
                PlanAction::Replace => assert_eq!(std::fs::read(&live).unwrap(), b"new"),
                PlanAction::ReEnable => {
                    assert_eq!(std::fs::read(&live).unwrap(), b"pack bytes");
                    assert!(!disabled_path(&root, &entry.path).exists());
                }
                PlanAction::ReplaceDisabled => unreachable!(),
            }
        }
    }
    assert_eq!(loads[0], loads[1], "session loads grew with the pack size");
    assert!(loads[0] <= 2, "{} session loads for one apply", loads[0]);
}

/// The single check is not a blind spot: a setup record replaced from outside
/// the app mid-apply still stops the apply before its next change, while a
/// rewrite of the same setup is re-validated and the apply carries on.
#[tokio::test]
async fn apply_stops_before_the_next_change_when_another_setup_replaces_the_record() {
    for replaced_by_foreign in [true, false] {
        let (_tmp, root, staging) = scaffold();
        for path in ["/config/a.txt", "/config/b.txt", "/config/c.txt"] {
            write_staged(&staging, path, b"created").await;
        }
        let owner = bind_setup(&root, &staging).await;
        let session = super::Session::require(&root, &owner).await.unwrap();
        let rewritten = if replaced_by_foreign {
            foreign(&session)
        } else {
            session
        };
        let writer_root = root.clone();
        hook("publish", live_path(&root, "/config/a.txt"), move || {
            write_record(&writer_root, &rewritten)
        });
        let entries: Vec<_> = ["/config/a.txt", "/config/b.txt", "/config/c.txt"]
            .into_iter()
            .map(|path| PlanEntry {
                path: path.into(),
                action: PlanAction::Create,
                reason: PlanReason::PackUpdate,
            })
            .collect();
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let before = super::session::load_count::of(&root);
        let result = super::execute_bound_plan(
            &entries,
            &root,
            &staging,
            &target,
            None,
            &mut Vec::new(),
            Some(&owner),
            &Default::default(),
        )
        .await;
        assert_eq!(super::session::load_count::of(&root) - before, 2);
        assert!(live_path(&root, "/config/a.txt").exists());
        if replaced_by_foreign {
            let error = format!("{:#}", result.unwrap_err());
            assert!(error.contains("preserved for recovery"), "{error}");
            assert!(!live_path(&root, "/config/b.txt").exists());
            assert!(!live_path(&root, "/config/c.txt").exists());
        } else {
            result.unwrap();
            assert!(live_path(&root, "/config/c.txt").exists());
        }
    }
}

/// `count` untracked files ticked for removal in one recovery batch.
async fn many_cleanup_selections(root: &std::path::Path, count: usize) -> super::RepairMarkerFile {
    let mut options = cleanup_options(root, "/config/retired-000.cfg").await;
    let template = options.cleanup_selections[0].clone();
    for i in 1..count {
        let path = format!("/config/retired-{i:03}.cfg");
        write_live(root, &path, b"retired").await;
        options
            .cleanup_selections
            .push(crate::managers::instance::modpack::CleanupSelection {
                path,
                ..template.clone()
            });
    }
    options
}

/// A pack that ships a world (a CurseForge world addon, or `saves` in its
/// overrides) stages it under `/saves`. Its setup must still load and verify.
#[tokio::test]
async fn setup_with_a_staged_world_loads_and_verifies() {
    let (_tmp, root, staging) = scaffold();
    write_staged(&staging, "/mods/a.jar", b"mod").await;
    write_staged(&staging, "/saves/FPS Test/level.dat", b"world").await;
    let owner = bind_setup(&root, &staging).await;
    let session = super::Session::require(&root, &owner).await.unwrap();
    assert_eq!(
        session.target.as_ref().unwrap().sources["/saves/FPS Test/level.dat"],
        "/saves/FPS Test/level.dat"
    );
    session
        .verify_sources(&staging, &root.join("instance"))
        .await
        .unwrap();
}

#[tokio::test]
async fn cleanup_checks_setup_ownership_once_for_all_selections() {
    let (_tmp, root, staging) = scaffold();
    write_staged(&staging, "/config/current.cfg", b"current").await;
    let owner = bind_setup(&root, &staging).await;
    write_live(&root, "/config/retired-000.cfg", b"retired").await;
    let options = many_cleanup_selections(&root, 100).await;
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let before = super::session::load_count::of(&root);
    let report = super::apply_bound_cleanup(
        &options,
        None,
        &target,
        &root,
        false,
        Some(&options.cleanup_selections[0].context),
        Some(&owner),
    )
    .await;
    assert_eq!(super::session::load_count::of(&root) - before, 1);
    assert_eq!(report.removed.len(), 100, "{:?}", report.notes);
    for selection in &options.cleanup_selections {
        assert!(!live_path(&root, &selection.path).exists());
    }
}

#[tokio::test]
async fn cleanup_preserves_every_selected_file_when_another_setup_owns_the_instance() {
    let (_tmp, root, staging) = scaffold();
    write_staged(&staging, "/config/current.cfg", b"current").await;
    let owner = bind_setup(&root, &staging).await;
    write_live(&root, "/config/retired-000.cfg", b"retired").await;
    let options = many_cleanup_selections(&root, 100).await;
    let session = super::Session::require(&root, &owner).await.unwrap();
    write_record(&root, &foreign(&session));
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let report = super::apply_bound_cleanup(
        &options,
        None,
        &target,
        &root,
        false,
        Some(&options.cleanup_selections[0].context),
        Some(&owner),
    )
    .await;
    assert!(report.removed.is_empty());
    assert!(
        report
            .notes
            .iter()
            .any(|note| note.starts_with("Cleanup preserved: setup ownership changed")),
        "{:?}",
        report.notes
    );
    for selection in &options.cleanup_selections {
        assert_eq!(
            std::fs::read(live_path(&root, &selection.path)).unwrap(),
            b"retired"
        );
    }
    let batch = root
        .join(".repair-recovery")
        .join(options.cleanup_batch.as_ref().unwrap());
    assert_eq!(std::fs::read_dir(batch).unwrap().count(), 0);
}

/// Run with `cargo test --release -p carbon_app -- --ignored
/// apply_of_a_5000_file_update`. An update where little changed pays for
/// every path of the pack, so the loop itself must stay linear.
#[tokio::test]
#[ignore = "timing benchmark; run in release"]
async fn apply_of_a_5000_file_update_finishes_quickly() {
    let (_tmp, root, staging) = scaffold();
    let mut entries = Vec::new();
    for i in 0..5000 {
        let path = format!("/config/f{i:04}.txt");
        write_staged(&staging, &path, b"same").await;
        let changed = i % 100 == 0;
        write_live(&root, &path, if changed { b"edit" } else { b"same" }).await;
        entries.push(PlanEntry {
            path,
            action: if changed {
                PlanAction::Replace
            } else {
                PlanAction::Keep
            },
            reason: PlanReason::PackUpdate,
        });
    }
    let owner = bind_setup(&root, &staging).await;
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let started = std::time::Instant::now();
    super::execute_bound_plan(
        &entries,
        &root,
        &staging,
        &target,
        None,
        &mut Vec::new(),
        Some(&owner),
        &Default::default(),
    )
    .await
    .unwrap();
    let elapsed = started.elapsed();
    assert!(elapsed < std::time::Duration::from_secs(2), "{elapsed:?}");
}

fn names(dir: impl AsRef<std::path::Path>) -> Vec<String> {
    let mut names: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A fresh install publishes every staged file as a hard link, as cheap as
/// 2.0.40's rename: no byte is copied, the staged original stays for a
/// resumed apply, and each destination folder is synced once, not each file.
#[tokio::test]
async fn fresh_install_links_every_file_and_syncs_each_folder_once() {
    let (_tmp, root, staging) = scaffold();
    let mut paths = Vec::new();
    for folder in ["/mods", "/config/pack", "/config/other"] {
        for i in 0..10 {
            let path = format!("{folder}/f{i}.txt");
            write_staged(&staging, &path, path.as_bytes()).await;
            paths.push(path);
        }
    }
    // Shipped disabled: it lands as the disabled twin.
    write_staged(&staging, "/mods/off.jar.disabled", b"off").await;
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let staged: HashSet<String> = target.files.keys().cloned().collect();
    let entries = apply_plan::plan(PlanInputs {
        old: None,
        target: &target,
        staged: &staged,
        disk: &HashMap::new(),
        coexisting_disabled_twin_md5: &HashMap::new(),
        mode: ApplyMode::VersionChange,
        fs_case_insensitive: false,
    })
    .unwrap();
    assert_eq!(entries.len(), 31);
    assert!(entries.iter().all(|e| e.action == PlanAction::Create));

    let outcomes = super::execute_plan(&entries, &root, &staging, &target)
        .await
        .unwrap();

    assert_eq!(outcomes, entries);
    for path in &paths {
        let live = live_path(&root, path);
        let source = staged_path(&staging, path);
        assert_eq!(std::fs::read(&live).unwrap(), path.as_bytes());
        assert!(
            same_file::is_same_file(&live, &source).unwrap(),
            "{path} was copied, not linked"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            assert_eq!(std::fs::metadata(&source).unwrap().nlink(), 2);
        }
    }
    assert!(
        same_file::is_same_file(
            disabled_path(&root, "/mods/off.jar"),
            staged_path(&staging, "/mods/off.jar.disabled")
        )
        .unwrap()
    );
    assert!(!live_path(&root, "/mods/off.jar").exists());
    // Nothing else, such as a private copy, was left beside them.
    assert_eq!(names(root.join("instance/mods")).len(), 11);
    assert_eq!(names(root.join("instance/config/pack")).len(), 10);
    assert_eq!(names(root.join("instance/config/other")).len(), 10);

    if cfg!(not(windows)) {
        let data = root.join("instance");
        let config = data.join("config");
        let expected: HashMap<PathBuf, usize> = [
            config.join("other"),
            config.join("pack"),
            config,
            data.join("mods"),
            data.clone(),
        ]
        .into_iter()
        .map(|folder| (folder, 1))
        .collect();
        assert_eq!(syncs_under(&data), expected);
    }
}

/// Volumes without hard links (FAT32, exFAT, some network shares): the
/// apply moves each staged file into place, as 2.0.40 did, instead of
/// copying it. No byte is written again and the pack is on the drive once,
/// a folder that refused one link isn't asked again, and every folder is
/// synced once, the staging folders the files left included. A Create still
/// never replaces a file that arrived first.
#[tokio::test]
async fn volumes_without_hard_links_move_staged_files_as_2_0_40_did() {
    let (_tmp, root, staging) = scaffold();
    let data = root.join("instance");
    without_hard_links(root.clone());
    const CREATE: [&str; 3] = ["/mods/a.jar", "/mods/b.jar", "/mods/c.jar"];
    const LATE: &str = "/mods/late.jar";
    const REPLACE: &str = "/config/a.cfg";
    const REPLACE_DISABLED: &str = "/mods/off.jar";
    const SHIPPED_DISABLED: &str = "/mods/shipped.jar";
    for path in CREATE.into_iter().chain([LATE]) {
        write_staged(&staging, path, path.as_bytes()).await;
    }
    write_staged(&staging, REPLACE, b"new config").await;
    write_live(&root, REPLACE, b"old config").await;
    write_staged(&staging, REPLACE_DISABLED, b"new off").await;
    write_disabled(&root, REPLACE_DISABLED, b"old off").await;
    write_staged(&staging, "/mods/shipped.jar.disabled", b"shipped off").await;
    let late = live_path(&root, LATE);
    let written = late.clone();
    hook("publish", late.clone(), move || {
        std::fs::write(written, b"late user file").unwrap()
    });
    let entry = |path: &str, action: PlanAction| PlanEntry {
        path: path.into(),
        action,
        reason: PlanReason::PackUpdate,
    };
    let mut entries: Vec<_> = CREATE
        .into_iter()
        .map(|path| entry(path, PlanAction::Create))
        .collect();
    entries.extend([
        entry(LATE, PlanAction::Create),
        entry(SHIPPED_DISABLED, PlanAction::Create),
        entry(REPLACE_DISABLED, PlanAction::ReplaceDisabled),
        entry(REPLACE, PlanAction::Replace),
    ]);

    let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();

    for (outcome, entry) in outcomes.iter().zip(&entries) {
        if entry.path == LATE {
            assert_eq!(outcome.action, PlanAction::Keep);
            assert_eq!(outcome.reason, PlanReason::PreservedExisting);
        } else {
            assert_eq!(outcome, entry);
        }
    }
    for path in CREATE {
        assert_eq!(
            std::fs::read(live_path(&root, path)).unwrap(),
            path.as_bytes()
        );
        assert!(!staged_path(&staging, path).exists(), "{path} was copied");
    }
    assert_eq!(std::fs::read(&late).unwrap(), b"late user file");
    assert_eq!(
        std::fs::read(staged_path(&staging, LATE)).unwrap(),
        LATE.as_bytes()
    );
    assert_eq!(
        std::fs::read(live_path(&root, REPLACE)).unwrap(),
        b"new config"
    );
    assert_eq!(
        std::fs::read(disabled_path(&root, REPLACE_DISABLED)).unwrap(),
        b"new off"
    );
    assert_eq!(
        std::fs::read(disabled_path(&root, SHIPPED_DISABLED)).unwrap(),
        b"shipped off"
    );
    assert_eq!(
        names(staging.join("instance/mods")),
        ["late.jar"],
        "only the file that stayed out is left in staging"
    );
    assert!(names(staging.join("instance/config")).is_empty());
    // Nothing else, such as a private copy or link name, was left beside them.
    assert_eq!(
        names(data.join("mods")),
        [
            "a.jar",
            "b.jar",
            "c.jar",
            "late.jar",
            "off.jar.disabled",
            "shipped.jar.disabled"
        ]
    );
    assert_eq!(names(data.join("config")), ["a.cfg"]);
    // One refused link per folder; the rest of it went straight to moves.
    assert_eq!(attempts_under("link", &data.join("mods")), 1);
    assert_eq!(attempts_under("link", &data.join("config")), 1);
    assert_eq!(attempts_under("move", &data.join("mods")), 6);

    if cfg!(not(windows)) {
        let once = |folders: Vec<PathBuf>| -> HashMap<PathBuf, usize> {
            folders.into_iter().map(|folder| (folder, 1)).collect()
        };
        assert_eq!(
            syncs_under(&data),
            once(vec![data.join("mods"), data.join("config"), data.clone()])
        );
        assert_eq!(
            syncs_under(&staging),
            once(vec![
                staging.join("instance/mods"),
                staging.join("instance/config")
            ])
        );
    }
}

/// A folder on another drive (a mods folder linked there), which neither a
/// link nor a move from staging reaches, gets copies: each checked against
/// the target hash, with the staged original kept until the setup commits.
/// One refused move is enough for the folder, and it is synced once, not
/// once per file.
#[tokio::test]
async fn folders_on_another_drive_get_verified_copies() {
    let (_tmp, root, staging) = scaffold();
    let data = root.join("instance");
    without_hard_links(root.clone());
    on_another_drive(data.join("mods"));
    const CREATE: [&str; 3] = ["/mods/a.jar", "/mods/b.jar", "/mods/c.jar"];
    const REPLACE: &str = "/config/a.cfg";
    for path in CREATE {
        write_staged(&staging, path, path.as_bytes()).await;
    }
    write_staged(&staging, REPLACE, b"new config").await;
    write_live(&root, REPLACE, b"old config").await;
    let mut entries: Vec<_> = CREATE
        .into_iter()
        .map(|path| PlanEntry {
            path: path.into(),
            action: PlanAction::Create,
            reason: PlanReason::PackUpdate,
        })
        .collect();
    entries.push(PlanEntry {
        path: REPLACE.into(),
        action: PlanAction::Replace,
        reason: PlanReason::PackUpdate,
    });

    let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();

    assert_eq!(outcomes, entries);
    for path in CREATE {
        let live = live_path(&root, path);
        assert_eq!(std::fs::read(&live).unwrap(), path.as_bytes());
        assert_eq!(
            std::fs::read(staged_path(&staging, path)).unwrap(),
            path.as_bytes()
        );
        assert!(!same_file::is_same_file(&live, staged_path(&staging, path)).unwrap());
    }
    // The config folder is on the staging drive: moved.
    assert_eq!(
        std::fs::read(live_path(&root, REPLACE)).unwrap(),
        b"new config"
    );
    assert!(!staged_path(&staging, REPLACE).exists());
    assert_eq!(names(data.join("mods")), ["a.jar", "b.jar", "c.jar"]);
    assert_eq!(attempts_under("link", &data.join("mods")), 1);
    assert_eq!(attempts_under("move", &data.join("mods")), 1);
    if cfg!(not(windows)) {
        let expected: HashMap<PathBuf, usize> =
            [data.join("mods"), data.join("config"), data.clone()]
                .into_iter()
                .map(|folder| (folder, 1))
                .collect();
        assert_eq!(syncs_under(&data), expected);
    }

    // A staged file that no longer matches its verified hash is refused.
    const CHANGED: &str = "/mods/changed.jar";
    write_staged(&staging, CHANGED, b"changed after verification").await;
    let mut target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    target.files.get_mut(CHANGED).unwrap().sha512 = sha2::Sha512::digest(b"verified bytes").into();
    let error = super::execute_plan(
        &[PlanEntry {
            path: CHANGED.into(),
            action: PlanAction::Create,
            reason: PlanReason::PackUpdate,
        }],
        &root,
        &staging,
        &target,
    )
    .await
    .unwrap_err();
    assert!(
        format!("{error:#}").contains("staged content changed before publication"),
        "{error:#}"
    );
    assert!(!live_path(&root, CHANGED).exists());
    assert_eq!(
        std::fs::read(staged_path(&staging, CHANGED)).unwrap(),
        b"changed after verification"
    );
}

/// A link refused for any other reason, on a volume that has links, still
/// publishes: by a move, as before the link existed.
#[tokio::test]
async fn other_link_refusals_fall_back_to_a_move() {
    use std::io::ErrorKind;
    for kind in [ErrorKind::Unsupported, ErrorKind::PermissionDenied] {
        let (_tmp, root, staging) = scaffold();
        const CREATE: &str = "/mods/new.jar";
        const REPLACE: &str = "/config/a.cfg";
        write_staged(&staging, CREATE, b"new mod").await;
        write_staged(&staging, REPLACE, b"new config").await;
        write_live(&root, REPLACE, b"old config").await;
        let entries = vec![
            PlanEntry {
                path: REPLACE.into(),
                action: PlanAction::Replace,
                reason: PlanReason::PackUpdate,
            },
            PlanEntry {
                path: CREATE.into(),
                action: PlanAction::Create,
                reason: PlanReason::PackUpdate,
            },
        ];
        fail_link(live_path(&root, CREATE), kind);
        fail_link(live_path(&root, REPLACE), kind);

        let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();

        assert_eq!(outcomes, entries);
        for (path, bytes) in [(CREATE, b"new mod" as &[u8]), (REPLACE, b"new config")] {
            let live = live_path(&root, path);
            assert!(link_fault(&live).is_none(), "{path} never tried to link");
            assert_eq!(std::fs::read(&live).unwrap(), bytes);
            assert!(!staged_path(&staging, path).exists());
        }
        assert_eq!(names(root.join("instance/config")), ["a.cfg"]);
    }
}

/// Replace and ReplaceDisabled publish through a private second name that
/// never ends in `.jar`. A failure between the link and the rename keeps
/// the old file, the staged source and nothing else; the retry finishes.
#[tokio::test]
async fn replacements_resume_after_a_failure_between_link_and_rename() {
    for action in [PlanAction::Replace, PlanAction::ReplaceDisabled] {
        let (_tmp, root, staging) = scaffold();
        const PATH: &str = "/mods/b.jar";
        let destination = if action == PlanAction::Replace {
            write_live(&root, PATH, b"old pack bytes").await;
            live_path(&root, PATH)
        } else {
            write_disabled(&root, PATH, b"old pack bytes").await;
            disabled_path(&root, PATH)
        };
        let name = destination
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        write_staged(&staging, PATH, b"new pack bytes").await;
        let entries = vec![PlanEntry {
            path: PATH.into(),
            action: action.clone(),
            reason: PlanReason::PackUpdate,
        }];

        fail_io("link-rename", destination.clone());
        assert!(execute_plan(&entries, &root, &staging).await.is_err());
        assert_eq!(std::fs::read(&destination).unwrap(), b"old pack bytes");
        assert_eq!(
            std::fs::read(staged_path(&staging, PATH)).unwrap(),
            b"new pack bytes"
        );
        assert_eq!(names(root.join("instance/mods")), [name.clone()]);

        let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();
        assert_eq!(outcomes, entries);
        assert_eq!(std::fs::read(&destination).unwrap(), b"new pack bytes");
        assert!(same_file::is_same_file(&destination, staged_path(&staging, PATH)).unwrap());
        assert_eq!(names(root.join("instance/mods")), [name.clone()]);

        // A retry that finds the destination already is the staged file,
        // still holding the pack's bytes, publishes it as planned: a match
        // alone (a 64-bit file index on Windows) never turns a replacement
        // into a Keep. No private name is left beside it.
        let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();
        assert_eq!(outcomes, entries);
        assert_eq!(std::fs::read(&destination).unwrap(), b"new pack bytes");
        assert!(same_file::is_same_file(&destination, staged_path(&staging, PATH)).unwrap());
        assert_eq!(names(root.join("instance/mods")), [name]);
    }
}

/// An `md5` the way the planner and the audit record it.
fn md5_of(bytes: &[u8]) -> [u8; 16] {
    use md5::Digest as _;
    md5::Md5::digest(bytes).into()
}

/// A repair or update linked a file, was interrupted, and the file was then
/// edited in place, which edits the staged original too. The resumed
/// replacement finds no pack copy left to publish: the edit is kept and the
/// audit says so instead of "already present", with a way to get the pack's
/// version back.
#[tokio::test]
async fn a_replacement_whose_linked_source_was_edited_in_place_keeps_the_edit() {
    const PATH: &str = "/config/x.json";
    const PACK: &[u8] = b"pack bytes";
    const EDITED: &[u8] = b"edited in place";
    for action in [PlanAction::Replace, PlanAction::ReplaceDisabled] {
        let (_tmp, root, staging) = scaffold();
        write_staged(&staging, PATH, PACK).await;
        let target = super::packinfo::scan_dir(&staging.join("instance"), None)
            .await
            .unwrap();
        let destination = if action == PlanAction::Replace {
            live_path(&root, PATH)
        } else {
            disabled_path(&root, PATH)
        };
        std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
        std::fs::hard_link(staged_path(&staging, PATH), &destination).unwrap();
        std::fs::write(&destination, EDITED).unwrap();
        assert_eq!(std::fs::read(staged_path(&staging, PATH)).unwrap(), EDITED);
        let entries = vec![PlanEntry {
            path: PATH.into(),
            action: action.clone(),
            reason: PlanReason::RepairOverwrote {
                original: md5_of(PACK),
                current: md5_of(EDITED),
            },
        }];

        let mut notes = Vec::new();
        let outcomes = super::execute_bound_plan(
            &entries,
            &root,
            &staging,
            &target,
            None,
            &mut notes,
            None,
            &Default::default(),
        )
        .await
        .unwrap();

        assert_eq!(
            outcomes[0].reason,
            PlanReason::ModifiedByUser {
                original: md5_of(PACK),
                current: md5_of(EDITED),
            },
            "{action:?}"
        );
        assert_eq!(outcomes[0].action, PlanAction::Keep);
        assert_eq!(std::fs::read(&destination).unwrap(), EDITED);
        assert!(same_file::is_same_file(&destination, staged_path(&staging, PATH)).unwrap());
        assert_eq!(names(root.join("instance/config")).len(), 1);
        assert_eq!(notes.len(), 1);
        assert!(
            notes[0].starts_with("/config/x.json was changed while this setup was interrupted")
                && notes[0].ends_with("Use Repair Modpack to restore it."),
            "{notes:?}"
        );
        let audit = super::render_cleanup_audit(
            &outcomes,
            &super::CleanupReport {
                removed: Vec::new(),
                notes,
            },
        );
        assert!(
            audit.contains("/config/x.json: modified by user"),
            "{audit}"
        );
        assert!(!audit.contains("already present"), "{audit}");
    }
}

/// A resumed apply never publishes a staged source that changed after the
/// apply started, whatever the plan wants from it. A Create keeps the file
/// absent, a replacement keeps the live file, and both say how to restore
/// the pack's version. The staged originals stay untouched.
#[tokio::test]
async fn changed_staged_sources_are_never_published() {
    const CREATE: &str = "/config/created.json";
    const REPLACE: &str = "/config/replaced.json";
    const DISABLED: &str = "/mods/off.jar";
    let (_tmp, root, staging) = scaffold();
    write_staged(&staging, CREATE, b"edited create").await;
    write_staged(&staging, REPLACE, b"edited replace").await;
    write_staged(&staging, DISABLED, b"edited disabled").await;
    write_live(&root, REPLACE, b"old replace").await;
    write_disabled(&root, DISABLED, b"old disabled").await;
    let target = packinfo(&[(CREATE, 1), (REPLACE, 2), (DISABLED, 3)]);
    let entries = vec![
        PlanEntry {
            path: CREATE.into(),
            action: PlanAction::Create,
            reason: PlanReason::RepairRestored,
        },
        PlanEntry {
            path: REPLACE.into(),
            action: PlanAction::Replace,
            reason: PlanReason::PackUpdate,
        },
        PlanEntry {
            path: DISABLED.into(),
            action: PlanAction::ReplaceDisabled,
            reason: PlanReason::PackUpdate,
        },
    ];
    let sources = super::session::SourceCheck {
        published: Vec::new(),
        changed: [CREATE, REPLACE, DISABLED]
            .into_iter()
            .map(String::from)
            .collect(),
    };

    let mut notes = Vec::new();
    let outcomes = super::execute_bound_plan(
        &entries, &root, &staging, &target, None, &mut notes, None, &sources,
    )
    .await
    .unwrap();

    assert_eq!(
        outcomes
            .iter()
            .map(|e| (e.path.as_str(), e.action.clone(), e.reason.clone()))
            .collect::<Vec<_>>(),
        [
            (CREATE, PlanAction::Keep, PlanReason::DeletedByUser),
            (
                REPLACE,
                PlanAction::Keep,
                PlanReason::ModifiedByUser {
                    original: [2; 16],
                    current: md5_of(b"old replace"),
                },
            ),
            (
                DISABLED,
                PlanAction::Keep,
                PlanReason::ModifiedByUser {
                    original: [3; 16],
                    current: md5_of(b"old disabled"),
                },
            ),
        ]
    );
    assert!(!live_path(&root, CREATE).exists());
    assert_eq!(
        std::fs::read(live_path(&root, REPLACE)).unwrap(),
        b"old replace"
    );
    assert_eq!(
        std::fs::read(disabled_path(&root, DISABLED)).unwrap(),
        b"old disabled"
    );
    for (path, bytes) in [
        (CREATE, b"edited create" as &[u8]),
        (REPLACE, b"edited replace"),
        (DISABLED, b"edited disabled"),
    ] {
        assert_eq!(std::fs::read(staged_path(&staging, path)).unwrap(), bytes);
    }
    assert_eq!(notes.len(), 3, "{notes:?}");
    for path in [CREATE, REPLACE, DISABLED] {
        assert!(notes.iter().any(|note| note.starts_with(path)), "{notes:?}");
    }
}

/// A representation change (here a replacement the pack ships disabled)
/// whose staged source changed after the apply started keeps both live
/// files as they are instead of failing every retry.
#[tokio::test]
async fn a_representation_change_from_a_changed_source_is_preserved() {
    const PATH: &str = "/mods/b.jar";
    let (_tmp, root, staging) = scaffold();
    write_live(&root, PATH, b"old bare").await;
    write_staged(&staging, "/mods/b.jar.disabled", b"edited").await;
    let mut target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    target.files.get_mut(PATH).unwrap().sha512 = sha2::Sha512::digest(b"pack bytes").into();
    let entries = vec![PlanEntry {
        path: PATH.into(),
        action: PlanAction::Replace,
        reason: PlanReason::PackUpdate,
    }];

    // The second pass resumes the completed, conflicted transition record.
    for attempt in 0..2 {
        let mut notes = Vec::new();
        let outcomes = super::execute_bound_plan(
            &entries,
            &root,
            &staging,
            &target,
            None,
            &mut notes,
            None,
            &Default::default(),
        )
        .await
        .unwrap();

        assert_eq!(outcomes[0].action, PlanAction::Keep);
        assert_eq!(std::fs::read(live_path(&root, PATH)).unwrap(), b"old bare");
        assert!(!disabled_path(&root, PATH).exists());
        assert_eq!(
            std::fs::read(staged_path(&staging, "/mods/b.jar.disabled")).unwrap(),
            b"edited"
        );
        // A resumed apply keeps the plain note and its way out.
        assert_eq!(
            notes,
            [super::source_changed_note(PATH)],
            "attempt {attempt}"
        );
    }
}

/// Files an interrupted earlier attempt linked get their folders synced by
/// the attempt that finishes, even when it has nothing left to publish
/// there: committing deletes their staged names.
#[cfg(not(windows))]
#[tokio::test]
async fn folders_an_earlier_attempt_linked_into_are_synced_before_applied() {
    const EARLIER: &str = "/config/earlier/a.json";
    let (_tmp, root, staging) = scaffold();
    write_staged(&staging, EARLIER, b"a").await;
    let live = live_path(&root, EARLIER);
    std::fs::create_dir_all(live.parent().unwrap()).unwrap();
    std::fs::hard_link(staged_path(&staging, EARLIER), &live).unwrap();
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let entries = vec![PlanEntry {
        path: EARLIER.into(),
        action: PlanAction::Keep,
        reason: PlanReason::PreservedExisting,
    }];
    let sources = super::session::SourceCheck {
        published: vec![live.clone()],
        changed: BTreeSet::new(),
    };

    super::execute_bound_plan(
        &entries,
        &root,
        &staging,
        &target,
        None,
        &mut Vec::new(),
        None,
        &sources,
    )
    .await
    .unwrap();

    let data = root.join("instance");
    let expected: HashMap<PathBuf, usize> = [
        data.join("config/earlier"),
        data.join("config"),
        data.clone(),
    ]
    .into_iter()
    .map(|folder| (folder, 1))
    .collect();
    assert_eq!(syncs_under(&data), expected);
}

/// A crash between a replacement's link and its rename leaves the private
/// name behind. It sits next to its destination, is hidden on Unix and never
/// looks like a mod or a disabled mod, and every attempt picks a new one.
#[test]
fn private_link_names_are_hidden_siblings_that_are_never_mods() {
    for destination in ["/i/mods/b.jar", "/i/mods/b.jar.disabled", "/i/config/a.cfg"] {
        let destination = std::path::Path::new(destination);
        let first = super::private_link_path(destination).unwrap();
        assert_ne!(first, super::private_link_path(destination).unwrap());
        assert_eq!(first.parent(), destination.parent());
        let name = first.file_name().unwrap().to_str().unwrap();
        assert!(name.starts_with('.'), "{name}");
        for mod_suffix in [".jar", ".zip", ".disabled"] {
            assert!(!name.ends_with(mod_suffix), "{name}");
        }
    }
}

/// GPT-0215: a replacement the pack ships disabled, interrupted before the
/// old bare file moves out, resumes from its staged source, which is still
/// there.
#[tokio::test]
async fn disabled_replacement_interrupted_before_the_bare_file_moves_resumes() {
    use crate::util::publish::fault::{IN_USE, Step, inject, pending};
    let (_tmp, root, staging) = scaffold();
    const PATH: &str = "/mods/b.jar";
    const STAGED: &str = "/mods/b.jar.disabled";
    write_live(&root, PATH, b"old bare").await;
    write_staged(&staging, STAGED, b"new disabled").await;
    let transition = super::transition_path(&root, PATH);
    let recovery = root.join(".repair-recovery");
    let payload = std::sync::Arc::new(std::sync::Mutex::new(None));
    let injected = payload.clone();
    hook("remove-bare", live_path(&root, PATH), move || {
        let record: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&transition).unwrap()).unwrap();
        let payload = recovery
            .join(record["recovery_batch"].as_str().unwrap())
            .join(super::record_key(PATH))
            .join("payload");
        inject(Step::NoReplace, &payload, IN_USE);
        *injected.lock().unwrap() = Some(payload);
    });
    let entries = vec![PlanEntry {
        path: PATH.into(),
        action: PlanAction::Replace,
        reason: PlanReason::PackUpdate,
    }];

    assert!(execute_plan(&entries, &root, &staging).await.is_err());
    // The bare file's move into recovery is what failed.
    let payload = payload.lock().unwrap().take().expect("the hook ran");
    assert!(!pending(Step::NoReplace, &payload));
    assert_eq!(std::fs::read(live_path(&root, PATH)).unwrap(), b"old bare");
    assert_eq!(
        std::fs::read(staged_path(&staging, STAGED)).unwrap(),
        b"new disabled"
    );

    let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();
    assert_eq!(outcomes, entries);
    assert!(!live_path(&root, PATH).exists());
    assert_eq!(
        std::fs::read(disabled_path(&root, PATH)).unwrap(),
        b"new disabled"
    );
    assert_eq!(
        std::fs::read(staged_path(&staging, STAGED)).unwrap(),
        b"new disabled"
    );
}

/// Destinations past 260 characters (Windows MAX_PATH) publish by link too,
/// both a Create and a replacement through its private sibling.
#[tokio::test]
async fn long_destinations_are_linked_into_place() {
    let (_tmp, root, staging) = long_scaffold();
    let deep = format!("/kubejs/{}/{}", "c".repeat(40), "d".repeat(40));
    let create = format!("{deep}/created.json");
    let replace = format!("{deep}/replaced.json");
    write_staged(&staging, &create, b"created").await;
    write_staged(&staging, &replace, b"new pack bytes").await;
    write_live(&root, &replace, b"old pack bytes").await;
    assert!(live_path(&root, &replace).as_os_str().len() >= 270);
    let entries = vec![
        PlanEntry {
            path: create.clone(),
            action: PlanAction::Create,
            reason: PlanReason::PackUpdate,
        },
        PlanEntry {
            path: replace.clone(),
            action: PlanAction::Replace,
            reason: PlanReason::PackUpdate,
        },
    ];

    let outcomes = execute_plan(&entries, &root, &staging).await.unwrap();

    assert_eq!(outcomes, entries);
    for (path, bytes) in [
        (&create, b"created" as &[u8]),
        (&replace, b"new pack bytes"),
    ] {
        let live = live_path(&root, path);
        assert_eq!(std::fs::read(&live).unwrap(), bytes);
        assert!(same_file::is_same_file(&live, staged_path(&staging, path)).unwrap());
    }
    assert_eq!(
        names(live_path(&root, &deep)),
        ["created.json", "replaced.json"]
    );
}

/// Run with `cargo test --release -p carbon_app -- --ignored
/// fresh_install_of_3000_files`. Linking a fresh install costs about what
/// 2.0.40's renames did; copying, hashing and flushing every file did not.
#[tokio::test]
#[ignore = "timing benchmark; run in release"]
async fn fresh_install_of_3000_files_applies_about_as_fast_as_renaming() {
    const FILES: usize = 3000;
    let bytes = vec![7u8; 64 * 1024];
    let paths: Vec<_> = (0..FILES)
        .map(|i| format!("/config/{:02}/f{i:04}.bin", i % 30))
        .collect();

    // 2.0.40: one rename per staged file.
    let (_renamed_tmp, root, staging) = scaffold();
    for path in &paths {
        write_staged(&staging, path, &bytes).await;
    }
    let started = std::time::Instant::now();
    for path in &paths {
        let live = live_path(&root, path);
        tokio::fs::create_dir_all(live.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::rename(staged_path(&staging, path), live)
            .await
            .unwrap();
    }
    let renamed = started.elapsed();

    let (_linked_tmp, root, staging) = scaffold();
    for path in &paths {
        write_staged(&staging, path, &bytes).await;
    }
    let target = super::packinfo::scan_dir(&staging.join("instance"), None)
        .await
        .unwrap();
    let entries: Vec<_> = paths
        .iter()
        .map(|path| PlanEntry {
            path: path.clone(),
            action: PlanAction::Create,
            reason: PlanReason::PackUpdate,
        })
        .collect();
    let started = std::time::Instant::now();
    super::execute_plan(&entries, &root, &staging, &target)
        .await
        .unwrap();
    let linked = started.elapsed();

    eprintln!("renamed in {renamed:?}, linked in {linked:?}");
    assert!(
        linked < renamed * 10 + std::time::Duration::from_millis(500),
        "renamed in {renamed:?}, linked in {linked:?}"
    );
}

/// A download of `relative` (under `current`) that carries a checksum, as
/// every CurseForge and Modrinth pack file does.
fn checked_download(current: &std::path::Path, relative: &str) -> carbon_net::Downloadable {
    carbon_net::Downloadable::new("https://pack.invalid/file", current.join(relative))
        .with_checksum(Some(carbon_net::Checksum::Sha1("0".repeat(40))))
}

fn write_file(path: &std::path::Path, bytes: &[u8]) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, bytes).unwrap();
}

/// A retry finds what an interrupted attempt already downloaded in place, and
/// that attempt's folder goes: only its checksummed downloads are kept, never
/// its overrides or a file this attempt already holds.
#[tokio::test]
async fn reuse_moves_earlier_downloads_and_removes_their_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let setup = tmp.path().join(".setup");
    let earlier = setup.join("preparing-earlier");
    let older = setup.join("preparing-older");
    let current = setup.join("preparing-current");
    write_file(&earlier.join("instance/mods/a.jar"), b"downloaded");
    write_file(&earlier.join("instance/mods/unchecked.jar"), b"unchecked");
    write_file(
        &earlier.join("instance/config/override.json"),
        b"old override",
    );
    write_file(
        &earlier.join("instance/mods/extracted.jar"),
        b"earlier copy",
    );
    write_file(&older.join("instance/mods/b.jar"), b"older download");
    // This attempt extracted its own copy already.
    write_file(
        &current.join("instance/mods/extracted.jar"),
        b"this attempt's",
    );
    let downloads = vec![
        checked_download(&current, "instance/mods/a.jar"),
        checked_download(&current, "instance/mods/b.jar"),
        checked_download(&current, "instance/mods/extracted.jar"),
        checked_download(&current, "instance/mods/missing.jar"),
        carbon_net::Downloadable::new(
            "https://pack.invalid/file",
            current.join("instance/mods/unchecked.jar"),
        ),
    ];

    let reused = super::reuse_earlier_downloads(&setup, &current, &downloads).await;

    assert_eq!(reused, 2);
    assert_eq!(
        std::fs::read(current.join("instance/mods/a.jar")).unwrap(),
        b"downloaded"
    );
    assert_eq!(
        std::fs::read(current.join("instance/mods/b.jar")).unwrap(),
        b"older download"
    );
    assert_eq!(
        std::fs::read(current.join("instance/mods/extracted.jar")).unwrap(),
        b"this attempt's"
    );
    assert!(!current.join("instance/mods/unchecked.jar").exists());
    assert!(!current.join("instance/config/override.json").exists());
    assert!(!earlier.exists() && !older.exists());

    // Nothing to reuse still clears what earlier attempts left.
    let stale = setup.join("preparing-stale");
    write_file(&stale.join("instance/config/override.json"), b"stale");
    assert_eq!(
        super::reuse_earlier_downloads(&setup, &current, &[]).await,
        0
    );
    assert!(!stale.exists());
    assert!(current.join("instance/mods/a.jar").is_file());
}

/// A staging beside an unfinished attempt (2.0.40's, or the one a
/// re-preparation replaces) lends its downloads by link and stays as it was.
#[tokio::test]
async fn reuse_links_from_staging_and_leaves_it_untouched() {
    let tmp = tempfile::tempdir().unwrap();
    let setup = tmp.path().join(".setup");
    let staging = setup.join("staging");
    let current = setup.join("preparing-current");
    write_file(&staging.join("instance/mods/a.jar"), b"staged");
    std::fs::create_dir_all(current.join("instance")).unwrap();
    let downloads = vec![checked_download(&current, "instance/mods/a.jar")];

    assert_eq!(
        super::reuse_earlier_downloads(&setup, &current, &downloads).await,
        1
    );
    assert_eq!(
        std::fs::read(current.join("instance/mods/a.jar")).unwrap(),
        b"staged"
    );
    assert_eq!(
        std::fs::read(staging.join("instance/mods/a.jar")).unwrap(),
        b"staged"
    );
    assert!(staging.is_dir());
}

/// A folder in an earlier attempt that is a link is never followed, so a
/// reuse can't move a file from outside the setup.
#[cfg(unix)]
#[tokio::test]
async fn reuse_never_follows_a_link_out_of_the_setup() {
    let tmp = tempfile::tempdir().unwrap();
    let setup = tmp.path().join(".setup");
    let outside = tmp.path().join("outside");
    let earlier = setup.join("preparing-earlier");
    let current = setup.join("preparing-current");
    write_file(&outside.join("mods/a.jar"), b"not the launcher's");
    std::fs::create_dir_all(&earlier).unwrap();
    std::os::unix::fs::symlink(&outside, earlier.join("instance")).unwrap();
    std::fs::create_dir_all(current.join("instance")).unwrap();
    let downloads = vec![checked_download(&current, "instance/mods/a.jar")];

    assert_eq!(
        super::reuse_earlier_downloads(&setup, &current, &downloads).await,
        0
    );
    assert_eq!(
        std::fs::read(outside.join("mods/a.jar")).unwrap(),
        b"not the launcher's"
    );
    assert!(!current.join("instance/mods/a.jar").exists());
}

/// A file a pack ships both enabled and as its `.disabled` twin keeps only
/// the bare spelling, whether it was staged or skipped as already installed.
/// A `.disabled` name whose bare form ends in `.disabled` too is a different
/// file and stays.
#[tokio::test]
async fn shadowed_disabled_twins_are_dropped_from_the_preparation() {
    let tmp = tempfile::tempdir().unwrap();
    let staged = tmp.path().join("instance");
    for name in [
        "mods/both.jar",
        "mods/both.jar.disabled",
        "mods/skipped.jar.disabled",
        "mods/only.jar.disabled",
        "mods/odd.jar.disabled",
        "mods/odd.jar.disabled.disabled",
    ] {
        write_file(&staged.join(name), name.as_bytes());
    }
    let mut files: Vec<String> = [
        // Skipped: already installed with the pack's bytes.
        "/mods/skipped.jar",
        "/mods/both.jar",
        "/mods/both.jar.disabled",
        "/mods/skipped.jar.disabled",
        "/mods/only.jar.disabled",
        "/mods/odd.jar.disabled",
        "/mods/odd.jar.disabled.disabled",
    ]
    .iter()
    .map(|name| name.to_string())
    .collect();

    super::drop_shadowed_disabled_twins(&staged, &mut files)
        .await
        .unwrap();

    assert_eq!(
        files,
        [
            "/mods/skipped.jar",
            "/mods/both.jar",
            "/mods/only.jar.disabled",
            "/mods/odd.jar.disabled",
            "/mods/odd.jar.disabled.disabled",
        ]
    );
    assert!(staged.join("mods/both.jar").is_file());
    assert!(!staged.join("mods/both.jar.disabled").exists());
    assert!(!staged.join("mods/skipped.jar.disabled").exists());
    assert!(staged.join("mods/only.jar.disabled").is_file());
    assert!(staged.join("mods/odd.jar.disabled.disabled").is_file());
}
