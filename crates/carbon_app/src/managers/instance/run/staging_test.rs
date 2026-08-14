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

use super::apply_plan::{
    self, ApplyMode, DiskState, PlanAction, PlanEntry, PlanInputs, PlanReason,
};
use super::disk_scan;
use super::packinfo::{FileHashes, PackInfo};
use super::{apply_user_cleanup, execute_plan, render_audit};

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
        matches!(disk_after.get(PATH), Some(DiskState::Disabled { .. })),
        "expected the path to scan as Disabled after landing at the twin spelling, got {:?}",
        disk_after.get(PATH)
    );

    // Re-planning as a later version change would (same target, nothing
    // freshly staged this time) must classify it Keep/DisabledByUser, not
    // error again.
    let entries2 = apply_plan::plan(PlanInputs {
        old: Some(&target),
        target: &target,
        staged: &HashSet::new(),
        disk: &disk_after,
        mode: ApplyMode::VersionChange,
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
    let removed = apply_user_cleanup(&[PATH.to_string()], None, &target, &instance_root).await;

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
    let removed = apply_user_cleanup(&[PATH.to_string()], None, &target, &instance_root).await;

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

    let target = packinfo(&[]);
    let removed = apply_user_cleanup(&[REQUESTED.to_string()], None, &target, &instance_root).await;

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
    let removed = apply_user_cleanup(&[PATH.to_string()], None, &target, &instance_root).await;

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
    let removed = apply_user_cleanup(&[PATH.to_string()], None, &target, &instance_root).await;

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
    let removed =
        apply_user_cleanup(&[PATH.to_string()], Some(&old), &target, &instance_root).await;

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

    let removed = apply_user_cleanup(&bad_paths, None, &target, &instance_root).await;

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

    let target = packinfo(&[]);
    let removed = apply_user_cleanup(
        &["/mods/escape/victim.txt".to_string()],
        None,
        &target,
        &instance_root,
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
    expected.push_str("\nFiles deleted:\n");
    expected.push_str(" - /mods/pack-dropped.jar\n");
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
