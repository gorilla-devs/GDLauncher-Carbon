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
use super::{apply_user_cleanup, execute_plan, finish_promoted_staging, render_audit};

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
        !staged_path(&staging_dir, PATH).exists(),
        "the staged copy must be moved, not merely copied"
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
