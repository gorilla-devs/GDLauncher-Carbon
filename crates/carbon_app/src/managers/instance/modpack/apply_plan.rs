//! Pure planner for applying a modpack version change (or repair) to an
//! instance's files.
//!
//! [`plan`] is a pure function: given what the instance looked like the last
//! time it was staged (`old`), what the new/target pack version wants
//! (`target`), which target-version files are already staged and ready to
//! be moved into place (`staged`), and what is actually on disk right now
//! (`disk`), it produces a deterministic list of [`PlanEntry`], sorted by
//! path, describing exactly what to do with every path. It never touches
//! the filesystem itself — callers execute the plan; this module only
//! decides it.
//!
//! The guiding invariant is "never silently lose or skip user data": a file
//! the user modified, disabled, or deleted is always preserved as-is, and a
//! target file we have no staged source for is a hard error rather than a
//! silent no-op (see [`PlanError`]).

use std::collections::{BTreeSet, HashMap, HashSet};

use super::packinfo::{FileHashes, PackInfo};

/// Which kind of apply this plan is for. `VersionChange` reconciles the
/// instance against a *new* pack version; `Repair` re-reconciles it against
/// the version already installed, to fix corruption without changing
/// content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplyMode {
    VersionChange,
    Repair { re_enable_disabled: bool },
}

/// What is actually present on disk for a given path, independent of what
/// the pack manifests say should be there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiskState {
    Present { md5: [u8; 16] },
    Missing,
    Disabled { md5: [u8; 16] },
}

/// What the planner decided to do with a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanAction {
    Replace,
    Create,
    Delete,
    Keep,
    ReplaceDisabled,
    ReEnable,
}

/// Why the planner made the [`PlanAction`] it did, for audit trails and UI
/// explanations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanReason {
    PackUpdate,
    Unchanged,
    ModifiedByUser {
        original: [u8; 16],
        current: [u8; 16],
    },
    DeletedByUser,
    DisabledByUser,
    InSaveFolder,
    PackDropped,
    DroppedButModified {
        original: [u8; 16],
        current: [u8; 16],
    },
    PreservedExisting,
    RepairOverwrote {
        original: [u8; 16],
        current: [u8; 16],
    },
    RepairRestored,
    ReEnabled,
}

/// One path's worth of decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanEntry {
    pub path: String,
    pub action: PlanAction,
    pub reason: PlanReason,
}

/// Everything [`plan`] needs to decide every path. Borrowed rather than
/// owned since the planner only reads these once, for the duration of a
/// single call.
#[derive(Debug)]
pub struct PlanInputs<'a> {
    /// The pack info last staged onto this instance, if it has ever been
    /// staged before. `None` means a from-scratch install.
    pub old: Option<&'a PackInfo>,
    /// The pack info for the version being applied.
    pub target: &'a PackInfo,
    /// Paths whose target-version bytes are already staged and ready to be
    /// moved into place.
    pub staged: &'a HashSet<String>,
    /// What's actually on disk right now, keyed by path.
    pub disk: &'a HashMap<String, DiskState>,
    pub mode: ApplyMode,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PlanError {
    #[error(
        "planner requires a staged source for {path} but none exists — refusing to continue rather than silently skipping"
    )]
    MissingStagedSource { path: String },
}

/// Decide what to do with every path in `old ∪ target`, deterministically
/// and without touching the filesystem. See the module docs for the
/// invariants; per-path decisions are delegated to [`decide_version_change`]
/// or [`decide_repair`] depending on [`ApplyMode`], except for `/saves`
/// paths, which this function decides itself before consulting either, to
/// protect existing save bytes without ever resurrecting a deleted world:
///
/// - Disk `Present` or `Disabled` (bytes already exist, in any state):
///   `Keep`/[`PlanReason::InSaveFolder`], always, in both modes — a save is
///   never overwritten, replaced, re-enabled, or deleted, no matter what
///   `old`/`target` say about it.
/// - Disk `Missing` and `old` already recorded the path: `Keep`/
///   [`PlanReason::InSaveFolder`], always, in both modes including
///   [`ApplyMode::Repair`] — a world the user deleted is never resurrected,
///   not even by repair (repair's "restore deleted" contract deliberately
///   excludes saves).
/// - Disk `Missing` and `old` never recorded the path (a from-scratch
///   install, or a pack version that newly ships this world): falls through
///   to the normal per-mode rows below, so a pack-staged world actually gets
///   created when staged (`Create`/[`PlanReason::PackUpdate`] under
///   [`ApplyMode::VersionChange`], `Create`/[`PlanReason::RepairRestored`]
///   under [`ApplyMode::Repair`]) — a hard [`PlanError::MissingStagedSource`]
///   when it isn't staged — instead of being silently promised in the target
///   packinfo and never written to disk.
pub fn plan(inputs: PlanInputs) -> Result<Vec<PlanEntry>, PlanError> {
    let PlanInputs {
        old,
        target,
        staged,
        disk,
        mode,
    } = inputs;

    let mut universe: BTreeSet<String> = BTreeSet::new();
    if let Some(old) = old {
        universe.extend(old.files.keys().cloned());
    }
    universe.extend(target.files.keys().cloned());

    let mut entries = Vec::with_capacity(universe.len());

    for path in &universe {
        let disk_state = disk.get(path).copied().unwrap_or(DiskState::Missing);
        let old_hashes = old.and_then(|o| o.files.get(path));
        let target_hashes = target.files.get(path);

        // Existing save bytes (Present or Disabled) are protected
        // unconditionally. A Missing save is only protected when `old`
        // already knew about it — otherwise it falls through to the normal
        // rows so a pack-staged world can actually be created.
        if path.starts_with("/saves") {
            let protect_missing_save = old_hashes.is_some();
            let protected = match disk_state {
                DiskState::Present { .. } | DiskState::Disabled { .. } => true,
                DiskState::Missing => protect_missing_save,
            };
            if protected {
                entries.push(PlanEntry {
                    path: path.clone(),
                    action: PlanAction::Keep,
                    reason: PlanReason::InSaveFolder,
                });
                continue;
            }
        }

        let (action, reason) = match &mode {
            ApplyMode::VersionChange => {
                decide_version_change(old_hashes, target_hashes, disk_state, staged, path)?
            }
            ApplyMode::Repair { re_enable_disabled } => decide_repair(
                old_hashes,
                target_hashes,
                disk_state,
                staged,
                path,
                *re_enable_disabled,
            )?,
        };

        entries.push(PlanEntry {
            path: path.clone(),
            action,
            reason,
        });
    }

    Ok(entries)
}

/// Decision procedure for [`ApplyMode::VersionChange`]: reconcile the
/// instance against a *new* pack version. Grouped by which of `old`/`target`
/// the path belongs to.
fn decide_version_change(
    old_hashes: Option<&FileHashes>,
    target_hashes: Option<&FileHashes>,
    disk_state: DiskState,
    staged: &HashSet<String>,
    path: &str,
) -> Result<(PlanAction, PlanReason), PlanError> {
    Ok(match (old_hashes, target_hashes) {
        // Path exists in both the last-staged version and the target
        // version.
        (Some(old_hashes), Some(target_hashes)) => match disk_state {
            // Disk already matches the target we're applying, even though it
            // differs from `old`: a version change interrupted after the new
            // bytes were written but before the new packinfo was promoted
            // resumes into exactly this state. There is nothing left to do —
            // checked before the modified-by-user arm below so a
            // crash-resumed apply doesn't get classified as a user edit with
            // a misleading original/current md5 pair in the audit (the
            // md5==old==target case is still handled below, by the
            // old_hashes.md5 == target_hashes.md5 arm — this one only
            // catches md5==target but old differs).
            DiskState::Present { md5 } if md5 == target_hashes.md5 && md5 != old_hashes.md5 => {
                (PlanAction::Keep, PlanReason::Unchanged)
            }
            DiskState::Present { md5 } if md5 != old_hashes.md5 => (
                PlanAction::Keep,
                PlanReason::ModifiedByUser {
                    original: old_hashes.md5,
                    current: md5,
                },
            ),
            // Pristine (disk matches what we last staged) and the
            // target ships identical bytes: nothing to do, even if the
            // path happens to be staged — don't churn the file for no
            // reason.
            DiskState::Present { .. } if old_hashes.md5 == target_hashes.md5 => {
                (PlanAction::Keep, PlanReason::Unchanged)
            }
            // Pristine, target genuinely differs, and the new bytes are
            // ready: this is a real pack update.
            DiskState::Present { .. } if staged.contains(path) => {
                (PlanAction::Replace, PlanReason::PackUpdate)
            }
            // Pristine, target differs, but nothing is staged for it —
            // leave the existing file alone rather than guess.
            DiskState::Present { .. } => (PlanAction::Keep, PlanReason::Unchanged),
            DiskState::Missing => (PlanAction::Keep, PlanReason::DeletedByUser),
            DiskState::Disabled { .. } => (PlanAction::Keep, PlanReason::DisabledByUser),
        },
        // Path existed in the last-staged version but the target
        // dropped it.
        (Some(old_hashes), None) => decide_dropped(old_hashes, disk_state),
        // Path is new in the target version — either `old` never
        // existed at all (from-scratch install) or it simply didn't
        // have this path.
        (None, Some(_)) => match disk_state {
            DiskState::Missing => {
                if staged.contains(path) {
                    (PlanAction::Create, PlanReason::PackUpdate)
                } else {
                    // Fail loud: never silently skip a file we have no
                    // staged source to produce.
                    return Err(PlanError::MissingStagedSource {
                        path: path.to_string(),
                    });
                }
            }
            // We have no record of this file, so we can't tell whether
            // it's unrelated user content or something else entirely —
            // never overwrite it.
            DiskState::Present { .. } => (PlanAction::Keep, PlanReason::PreservedExisting),
            // Creating the enabled path here would manufacture the
            // both-files (enabled + disabled) state.
            DiskState::Disabled { .. } => (PlanAction::Keep, PlanReason::PreservedExisting),
        },
        (None, None) => unreachable!("path is drawn from old ∪ target"),
    })
}

/// Shared by both modes for a path `old` had but `target` dropped: a crash
/// or interrupted apply can leave the pack record pointing at a stale
/// version, and repairing onto that stale version treats the drop exactly
/// like a normal version change would.
fn decide_dropped(old_hashes: &FileHashes, disk_state: DiskState) -> (PlanAction, PlanReason) {
    match disk_state {
        DiskState::Present { md5 } if md5 == old_hashes.md5 => {
            (PlanAction::Delete, PlanReason::PackDropped)
        }
        DiskState::Present { md5 } => (
            PlanAction::Keep,
            PlanReason::DroppedButModified {
                original: old_hashes.md5,
                current: md5,
            },
        ),
        DiskState::Missing => (PlanAction::Keep, PlanReason::DeletedByUser),
        // The stale disabled twin (e.g. `foo.jar.disabled` when the
        // pack drops `foo.jar`) is surfaced by the untracked-file
        // walk, not by this planner.
        DiskState::Disabled { .. } => (PlanAction::Keep, PlanReason::DisabledByUser),
    }
}

/// Decision procedure for [`ApplyMode::Repair`]: re-reconcile the instance
/// against the version already installed (`target` — repair never changes
/// content, only fixes corruption/staleness relative to it).
///
/// Paths present in `target` are decided purely against `target`'s hash,
/// regardless of what `old` says: repair re-installs the version already
/// recorded, so a path's relationship to `old` (e.g. "matches what we last
/// staged") is irrelevant — only its relationship to `target` (the record of
/// what *should* be there right now) matters. Only the stale `old \ target`
/// bucket (a crash left the record pointing at a previous version) still
/// consults `old`, via [`decide_dropped`] — identical to
/// [`ApplyMode::VersionChange`].
fn decide_repair(
    old_hashes: Option<&FileHashes>,
    target_hashes: Option<&FileHashes>,
    disk_state: DiskState,
    staged: &HashSet<String>,
    path: &str,
    re_enable_disabled: bool,
) -> Result<(PlanAction, PlanReason), PlanError> {
    let Some(target_hashes) = target_hashes else {
        let old_hashes = old_hashes.expect(
            "path is drawn from old ∪ target; a path absent from target must be present in old",
        );
        return Ok(decide_dropped(old_hashes, disk_state));
    };
    let target_md5 = target_hashes.md5;

    Ok(match disk_state {
        DiskState::Present { md5 } if md5 == target_md5 => {
            (PlanAction::Keep, PlanReason::Unchanged)
        }
        // Damaged (or stale) bytes on disk relative to target, with the
        // correct bytes staged: repair overwrites them rather than
        // preserving the damage.
        DiskState::Present { md5 } if staged.contains(path) => (
            PlanAction::Replace,
            PlanReason::RepairOverwrote {
                original: target_md5,
                current: md5,
            },
        ),
        DiskState::Present { .. } => {
            return Err(PlanError::MissingStagedSource {
                path: path.to_string(),
            });
        }
        DiskState::Missing if staged.contains(path) => {
            (PlanAction::Create, PlanReason::RepairRestored)
        }
        DiskState::Missing => {
            return Err(PlanError::MissingStagedSource {
                path: path.to_string(),
            });
        }
        // `re_enable_disabled` fully decides the Disabled case on its own:
        // when set, every disabled twin comes back regardless of the
        // no-re-enable rules below (staged required only if the bytes
        // actually need repairing); when unset, the file simply stays
        // disabled, repaired in place if damaged.
        DiskState::Disabled { md5 } if re_enable_disabled => {
            if md5 != target_md5 && !staged.contains(path) {
                return Err(PlanError::MissingStagedSource {
                    path: path.to_string(),
                });
            }
            (PlanAction::ReEnable, PlanReason::ReEnabled)
        }
        DiskState::Disabled { md5 } if md5 == target_md5 => {
            (PlanAction::Keep, PlanReason::DisabledByUser)
        }
        DiskState::Disabled { md5 } if staged.contains(path) => (
            PlanAction::ReplaceDisabled,
            PlanReason::RepairOverwrote {
                original: target_md5,
                current: md5,
            },
        ),
        DiskState::Disabled { .. } => {
            return Err(PlanError::MissingStagedSource {
                path: path.to_string(),
            });
        }
    })
}

#[cfg(test)]
mod test {
    use std::collections::{HashMap, HashSet};

    use super::*;
    use crate::managers::instance::modpack::packinfo::{FileHashes, PackInfo};

    const PATH: &str = "/mods/a.jar";
    // Used only by the "saves folder, disk missing" section further down;
    // shadowed harmlessly by the pre-existing function-local `SAVE_PATH`
    // constants in `saves_folder_is_always_kept` and
    // `repair_saves_folder_kept_even_when_damaged`, which stay on their own
    // literal and are otherwise untouched.
    const SAVE_PATH: &str = "/saves/w/level.dat";

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

    struct Case {
        name: &'static str,
        old: Option<&'static [(&'static str, u8)]>,
        target: &'static [(&'static str, u8)],
        staged: &'static [&'static str],
        disk: &'static [(&'static str, DiskCase)],
        mode: ApplyMode,
        expect: Expect,
    }

    #[derive(Clone, Copy)]
    enum DiskCase {
        Present(u8),
        Missing,
        Disabled(u8),
    }

    enum Expect {
        Entry(PlanAction, PlanReason),
        Error,
    }

    // Looks up `PATH` in the produced entries — every table case in this
    // module (bar the saves-specific ones, which check a `/saves` path
    // instead and so go through `run_for_path` directly) exercises exactly
    // one path.
    fn run(case: Case) {
        run_for_path(case, PATH);
    }

    fn run_for_path(case: Case, path: &str) {
        let old = case.old.map(packinfo);
        let target = packinfo(case.target);
        let staged: HashSet<String> = case.staged.iter().map(|s| s.to_string()).collect();
        let disk: HashMap<String, DiskState> = case
            .disk
            .iter()
            .map(|(path, disk_case)| {
                let state = match disk_case {
                    DiskCase::Present(seed) => DiskState::Present {
                        md5: hashes(*seed).md5,
                    },
                    DiskCase::Missing => DiskState::Missing,
                    DiskCase::Disabled(seed) => DiskState::Disabled {
                        md5: hashes(*seed).md5,
                    },
                };
                (path.to_string(), state)
            })
            .collect();

        let result = plan(PlanInputs {
            old: old.as_ref(),
            target: &target,
            staged: &staged,
            disk: &disk,
            mode: case.mode,
        });

        match case.expect {
            Expect::Entry(action, reason) => {
                let entries = result
                    .unwrap_or_else(|e| panic!("case '{}': expected Ok, got Err({e})", case.name));

                // Property, checked for every table case automatically:
                // the planner never produces more than one entry for the
                // same path — there's no scenario where e.g. a Create and a
                // Keep both exist for the same logical path ("never
                // both-files").
                let mut seen_paths = HashSet::new();
                for entry in &entries {
                    assert!(
                        seen_paths.insert(entry.path.clone()),
                        "case '{}': duplicate plan entry for path {}, got {entries:?}",
                        case.name,
                        entry.path
                    );
                }

                let entry = entries.iter().find(|e| e.path == path).unwrap_or_else(|| {
                    panic!("case '{}': no entry for {path}, got {entries:?}", case.name)
                });
                assert_eq!(
                    entry.action, action,
                    "case '{}': action mismatch",
                    case.name
                );
                assert_eq!(
                    entry.reason, reason,
                    "case '{}': reason mismatch",
                    case.name
                );
            }
            Expect::Error => {
                result
                    .err()
                    .unwrap_or_else(|| panic!("case '{}': expected Err, got Ok", case.name));
            }
        }
    }

    // --- old ∩ target ---------------------------------------------------

    #[test]
    fn both_pristine_staged_replaces() {
        run(Case {
            name: "both pristine + staged -> Replace/PackUpdate",
            old: Some(&[(PATH, 1)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Present(1))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Replace, PlanReason::PackUpdate),
        });
    }

    #[test]
    fn both_pristine_unstaged_keeps_unchanged() {
        run(Case {
            name: "both pristine + unstaged (hash unchanged) -> Keep/Unchanged",
            old: Some(&[(PATH, 1)]),
            target: &[(PATH, 1)],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(1))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::Unchanged),
        });
    }

    #[test]
    fn both_pristine_staged_identical_target_hash_keeps_unchanged() {
        run(Case {
            name: "both pristine + staged but target hash == old hash -> Keep/Unchanged",
            old: Some(&[(PATH, 1)]),
            target: &[(PATH, 1)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Present(1))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::Unchanged),
        });
    }

    #[test]
    fn both_disk_already_matches_target_keeps_unchanged() {
        // Crash-resume case: a version change that wrote the new bytes but
        // got interrupted before promoting packinfo leaves disk == target
        // but old still on record as the pre-change hash. Staged too, to
        // prove this arm is checked before the "pristine + staged ->
        // Replace/PackUpdate" arm as well as the modified-by-user one.
        run(Case {
            name: "old=1, target=2, disk == target (staged) -> Keep/Unchanged, not ModifiedByUser",
            old: Some(&[(PATH, 1)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Present(2))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::Unchanged),
        });
    }

    #[test]
    fn both_modified_keeps_modified_by_user() {
        run(Case {
            name: "both, disk modified relative to old -> Keep/ModifiedByUser",
            old: Some(&[(PATH, 1)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Present(9))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(
                PlanAction::Keep,
                PlanReason::ModifiedByUser {
                    original: hashes(1).md5,
                    current: hashes(9).md5,
                },
            ),
        });
    }

    #[test]
    fn both_missing_keeps_deleted_by_user() {
        run(Case {
            name: "both, disk missing -> Keep/DeletedByUser",
            old: Some(&[(PATH, 1)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Missing)],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::DeletedByUser),
        });
    }

    #[test]
    fn both_disabled_keeps_disabled_by_user() {
        run(Case {
            name: "both, disk disabled -> Keep/DisabledByUser",
            old: Some(&[(PATH, 1)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Disabled(1))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::DisabledByUser),
        });
    }

    // --- old \ target (dropped) ------------------------------------------

    #[test]
    fn drop_pristine_deletes_pack_dropped() {
        run(Case {
            name: "dropped, disk pristine -> Delete/PackDropped",
            old: Some(&[(PATH, 1)]),
            target: &[],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(1))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Delete, PlanReason::PackDropped),
        });
    }

    #[test]
    fn drop_modified_keeps_dropped_but_modified() {
        run(Case {
            name: "dropped, disk modified -> Keep/DroppedButModified",
            old: Some(&[(PATH, 1)]),
            target: &[],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(9))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(
                PlanAction::Keep,
                PlanReason::DroppedButModified {
                    original: hashes(1).md5,
                    current: hashes(9).md5,
                },
            ),
        });
    }

    #[test]
    fn drop_missing_keeps_deleted_by_user() {
        run(Case {
            name: "dropped, disk missing -> Keep/DeletedByUser",
            old: Some(&[(PATH, 1)]),
            target: &[],
            staged: &[],
            disk: &[(PATH, DiskCase::Missing)],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::DeletedByUser),
        });
    }

    #[test]
    fn drop_disabled_keeps_disabled_by_user() {
        run(Case {
            name: "dropped, disk disabled -> Keep/DisabledByUser",
            old: Some(&[(PATH, 1)]),
            target: &[],
            staged: &[],
            disk: &[(PATH, DiskCase::Disabled(1))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::DisabledByUser),
        });
    }

    // --- target \ old (new) ----------------------------------------------

    #[test]
    fn new_missing_staged_creates_pack_update() {
        run(Case {
            name: "new (old present but lacks path), missing + staged -> Create/PackUpdate",
            old: Some(&[]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Missing)],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Create, PlanReason::PackUpdate),
        });
    }

    #[test]
    fn new_missing_unstaged_errors() {
        run(Case {
            name: "new, missing + unstaged -> Error",
            old: Some(&[]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Missing)],
            mode: ApplyMode::VersionChange,
            expect: Expect::Error,
        });
    }

    #[test]
    fn new_present_keeps_preserved_existing() {
        run(Case {
            name: "new, disk present -> Keep/PreservedExisting",
            old: Some(&[]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(9))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::PreservedExisting),
        });
    }

    #[test]
    fn new_disabled_keeps_preserved_existing() {
        run(Case {
            name: "new, disk disabled -> Keep/PreservedExisting",
            old: Some(&[]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Disabled(9))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::PreservedExisting),
        });
    }

    // --- old = None (from-scratch install) --------------------------------

    #[test]
    fn old_none_missing_staged_creates() {
        run(Case {
            name: "old=None, missing + staged -> Create/PackUpdate",
            old: None,
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Create, PlanReason::PackUpdate),
        });
    }

    #[test]
    fn old_none_present_keeps_preserved_existing() {
        run(Case {
            name: "old=None, disk present -> Keep/PreservedExisting",
            old: None,
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(9))],
            mode: ApplyMode::VersionChange,
            expect: Expect::Entry(PlanAction::Keep, PlanReason::PreservedExisting),
        });
    }

    // --- saves folder + output shape --------------------------------------

    #[test]
    fn saves_folder_is_always_kept() {
        const SAVE_PATH: &str = "/saves/world/level.dat";
        let disk_states = [
            DiskState::Present { md5: hashes(1).md5 },
            DiskState::Missing,
            DiskState::Disabled { md5: hashes(1).md5 },
        ];
        let staged: HashSet<String> = HashSet::new();

        for disk_state in disk_states {
            let mut disk = HashMap::new();
            disk.insert(SAVE_PATH.to_string(), disk_state);

            // In both old and target.
            let old = packinfo(&[(SAVE_PATH, 1)]);
            let target = packinfo(&[(SAVE_PATH, 2)]);
            let entries = plan(PlanInputs {
                old: Some(&old),
                target: &target,
                staged: &staged,
                disk: &disk,
                mode: ApplyMode::VersionChange,
            })
            .unwrap_or_else(|e| panic!("saves folder entries must never error, got {e}"));
            let entry = entries
                .iter()
                .find(|e| e.path == SAVE_PATH)
                .expect("saves entry must be present (in both old and target)");
            assert_eq!(entry.action, PlanAction::Keep);
            assert_eq!(entry.reason, PlanReason::InSaveFolder);

            // Dropped: only in old.
            let old_dropped = packinfo(&[(SAVE_PATH, 1)]);
            let target_dropped = packinfo(&[]);
            let entries = plan(PlanInputs {
                old: Some(&old_dropped),
                target: &target_dropped,
                staged: &staged,
                disk: &disk,
                mode: ApplyMode::VersionChange,
            })
            .unwrap_or_else(|e| panic!("saves folder entries must never error, got {e}"));
            let entry = entries
                .iter()
                .find(|e| e.path == SAVE_PATH)
                .expect("saves entry must be present (dropped from target)");
            assert_eq!(entry.action, PlanAction::Keep);
            assert_eq!(entry.reason, PlanReason::InSaveFolder);
        }
    }

    #[test]
    fn output_is_sorted_by_path() {
        let target = packinfo(&[("/mods/z.jar", 1), ("/mods/a.jar", 2), ("/mods/m.jar", 3)]);
        let staged: HashSet<String> = ["/mods/z.jar", "/mods/a.jar", "/mods/m.jar"]
            .into_iter()
            .map(String::from)
            .collect();
        let disk: HashMap<String, DiskState> = HashMap::new();

        let entries = plan(PlanInputs {
            old: None,
            target: &target,
            staged: &staged,
            disk: &disk,
            mode: ApplyMode::VersionChange,
        })
        .expect("all staged, must not error");

        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["/mods/a.jar", "/mods/m.jar", "/mods/z.jar"]);
    }

    #[test]
    fn paths_outside_old_or_target_never_appear() {
        let target = packinfo(&[(PATH, 1)]);
        let staged: HashSet<String> = [PATH].into_iter().map(String::from).collect();
        let mut disk = HashMap::new();
        disk.insert(PATH.to_string(), DiskState::Missing);
        disk.insert(
            "/mods/ghost.jar".to_string(),
            DiskState::Present { md5: hashes(9).md5 },
        );

        let entries = plan(PlanInputs {
            old: None,
            target: &target,
            staged: &staged,
            disk: &disk,
            mode: ApplyMode::VersionChange,
        })
        .expect("staged present, must not error");

        assert_eq!(entries.len(), 1);
        assert!(entries.iter().all(|e| e.path != "/mods/ghost.jar"));
    }

    // --- Repair, re_enable_disabled: false, target-present rows ----------
    //
    // Decided purely against `target`'s hash, regardless of `old` — every
    // case below deliberately sets `old` to a *third*, distinct hash so a
    // planner that mistakenly compared against `old` (VersionChange-style)
    // instead of `target` would fail these.

    #[test]
    fn repair_pristine_keeps_unchanged() {
        run(Case {
            name: "repair, disk matches target (old is stale/different) -> Keep/Unchanged",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(2))],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Entry(PlanAction::Keep, PlanReason::Unchanged),
        });
    }

    #[test]
    fn repair_damaged_staged_replaces_overwrote() {
        run(Case {
            name: "repair, disk damaged vs target + staged -> Replace/RepairOverwrote",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Present(9))],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Entry(
                PlanAction::Replace,
                PlanReason::RepairOverwrote {
                    original: hashes(2).md5,
                    current: hashes(9).md5,
                },
            ),
        });
    }

    #[test]
    fn repair_damaged_unstaged_errors() {
        run(Case {
            name: "repair, disk damaged vs target + unstaged -> Error",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(9))],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Error,
        });
    }

    #[test]
    fn repair_missing_staged_creates_restored() {
        run(Case {
            name: "repair, disk missing + staged -> Create/RepairRestored",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Missing)],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Entry(PlanAction::Create, PlanReason::RepairRestored),
        });
    }

    #[test]
    fn repair_missing_unstaged_errors() {
        run(Case {
            name: "repair, disk missing + unstaged -> Error",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Missing)],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Error,
        });
    }

    #[test]
    fn repair_disabled_pristine_keeps_disabled_by_user() {
        run(Case {
            name: "repair(no re-enable), disabled twin matches target -> Keep/DisabledByUser",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Disabled(2))],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Entry(PlanAction::Keep, PlanReason::DisabledByUser),
        });
    }

    #[test]
    fn repair_disabled_damaged_staged_replaces_disabled_overwrote() {
        run(Case {
            name: "repair(no re-enable), disabled twin damaged + staged -> ReplaceDisabled/RepairOverwrote",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Disabled(9))],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Entry(
                PlanAction::ReplaceDisabled,
                PlanReason::RepairOverwrote {
                    original: hashes(2).md5,
                    current: hashes(9).md5,
                },
            ),
        });
    }

    #[test]
    fn repair_disabled_damaged_unstaged_errors() {
        run(Case {
            name: "repair(no re-enable), disabled twin damaged + unstaged -> Error",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Disabled(9))],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Error,
        });
    }

    // --- Repair, stale old \ target rows (crash left the record on a
    // --- previous version) — identical to VersionChange's dropped bucket,
    // --- regardless of `re_enable_disabled`.

    #[test]
    fn repair_stale_old_pristine_deletes_pack_dropped() {
        run(Case {
            name: "repair, target dropped path, disk pristine vs old -> Delete/PackDropped",
            old: Some(&[(PATH, 1)]),
            target: &[],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(1))],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Entry(PlanAction::Delete, PlanReason::PackDropped),
        });
    }

    #[test]
    fn repair_stale_old_modified_keeps_dropped_but_modified() {
        run(Case {
            name: "repair, target dropped path, disk modified vs old -> Keep/DroppedButModified",
            old: Some(&[(PATH, 1)]),
            target: &[],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(9))],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Entry(
                PlanAction::Keep,
                PlanReason::DroppedButModified {
                    original: hashes(1).md5,
                    current: hashes(9).md5,
                },
            ),
        });
    }

    #[test]
    fn repair_stale_old_missing_keeps_deleted_by_user() {
        run(Case {
            name: "repair, target dropped path, disk missing -> Keep/DeletedByUser",
            old: Some(&[(PATH, 1)]),
            target: &[],
            staged: &[],
            disk: &[(PATH, DiskCase::Missing)],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Entry(PlanAction::Keep, PlanReason::DeletedByUser),
        });
    }

    #[test]
    fn repair_stale_old_disabled_keeps_disabled_by_user() {
        run(Case {
            name: "repair, target dropped path, disk disabled -> Keep/DisabledByUser",
            old: Some(&[(PATH, 1)]),
            target: &[],
            staged: &[],
            disk: &[(PATH, DiskCase::Disabled(1))],
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            expect: Expect::Entry(PlanAction::Keep, PlanReason::DisabledByUser),
        });
    }

    // --- Repair, saves folder: kept even when the on-disk copy is damaged
    // --- relative to target, in either re_enable_disabled setting.

    #[test]
    fn repair_saves_folder_kept_even_when_damaged() {
        const SAVE_PATH: &str = "/saves/world/level.dat";
        let staged: HashSet<String> = HashSet::new();
        // All "damaged" relative to target (seed 2): present-but-wrong-hash,
        // missing entirely, or disabled-but-wrong-hash.
        let disk_states = [
            DiskState::Present { md5: hashes(9).md5 },
            DiskState::Missing,
            DiskState::Disabled { md5: hashes(9).md5 },
        ];

        for disk_state in disk_states {
            let mut disk = HashMap::new();
            disk.insert(SAVE_PATH.to_string(), disk_state);

            let old = packinfo(&[(SAVE_PATH, 1)]);
            let target = packinfo(&[(SAVE_PATH, 2)]);

            for re_enable_disabled in [false, true] {
                let entries = plan(PlanInputs {
                    old: Some(&old),
                    target: &target,
                    staged: &staged,
                    disk: &disk,
                    mode: ApplyMode::Repair { re_enable_disabled },
                })
                .unwrap_or_else(|e| {
                    panic!("saves folder entries must never error in repair mode, got {e}")
                });
                let entry = entries
                    .iter()
                    .find(|e| e.path == SAVE_PATH)
                    .expect("saves entry must be present");
                assert_eq!(entry.action, PlanAction::Keep);
                assert_eq!(entry.reason, PlanReason::InSaveFolder);
            }
        }
    }

    // --- saves folder, disk missing: create-vs-protect semantics ---------
    //
    // A `/saves` path only short-circuits to Keep/InSaveFolder when disk
    // holds existing bytes (Present/Disabled) or `old` already recorded the
    // path (a deleted world, never resurrected — see `plan`'s doc comment).
    // A path `old` never heard of, missing from disk, falls through to the
    // normal per-mode rows so a pack-staged world actually gets created
    // instead of being promised in packinfo and left behind in the staging
    // directory that gets deleted right after — the fresh-install
    // regression this section guards against.

    #[test]
    fn old_none_saves_missing_staged_creates_pack_update() {
        run_for_path(
            Case {
                name: "old=None, /saves path missing + staged -> Create/PackUpdate",
                old: None,
                target: &[(SAVE_PATH, 2)],
                staged: &[SAVE_PATH],
                disk: &[],
                mode: ApplyMode::VersionChange,
                expect: Expect::Entry(PlanAction::Create, PlanReason::PackUpdate),
            },
            SAVE_PATH,
        );
    }

    #[test]
    fn old_none_saves_missing_unstaged_errors() {
        run_for_path(
            Case {
                name: "old=None, /saves path missing + unstaged -> Error",
                old: None,
                target: &[(SAVE_PATH, 2)],
                staged: &[],
                disk: &[],
                mode: ApplyMode::VersionChange,
                expect: Expect::Error,
            },
            SAVE_PATH,
        );
    }

    #[test]
    fn saves_new_in_target_only_missing_staged_creates_pack_update() {
        run_for_path(
            Case {
                name: "/saves path new in target (old present but lacks path), missing + staged -> Create/PackUpdate",
                old: Some(&[]),
                target: &[(SAVE_PATH, 2)],
                staged: &[SAVE_PATH],
                disk: &[],
                mode: ApplyMode::VersionChange,
                expect: Expect::Entry(PlanAction::Create, PlanReason::PackUpdate),
            },
            SAVE_PATH,
        );
    }

    #[test]
    fn old_none_saves_missing_staged_repair_creates_restored() {
        // The imported-instance repair case: an instance with no staging
        // history at all (`old = None`) repaired against a pack version
        // that ships a world this instance has never had on disk. Missing +
        // `old` never recorded it falls through to the normal per-mode rows
        // (see the section doc above), so under Repair this is
        // Create/RepairRestored — the same fallthrough as the VersionChange
        // case above, just decided by `decide_repair` instead of
        // `decide_version_change`.
        run_for_path(
            Case {
                name: "old=None, /saves path missing + staged, Repair -> Create/RepairRestored",
                old: None,
                target: &[(SAVE_PATH, 2)],
                staged: &[SAVE_PATH],
                disk: &[],
                mode: ApplyMode::Repair {
                    re_enable_disabled: false,
                },
                expect: Expect::Entry(PlanAction::Create, PlanReason::RepairRestored),
            },
            SAVE_PATH,
        );
    }

    #[test]
    fn saves_in_old_and_target_missing_staged_version_change_keeps_in_save_folder() {
        run_for_path(
            Case {
                name: "/saves path in old ∩ target, missing + staged, VersionChange -> Keep/InSaveFolder (never resurrect)",
                old: Some(&[(SAVE_PATH, 1)]),
                target: &[(SAVE_PATH, 2)],
                staged: &[SAVE_PATH],
                disk: &[],
                mode: ApplyMode::VersionChange,
                expect: Expect::Entry(PlanAction::Keep, PlanReason::InSaveFolder),
            },
            SAVE_PATH,
        );
    }

    #[test]
    fn saves_in_old_and_target_missing_staged_repair_no_reenable_keeps_in_save_folder() {
        run_for_path(
            Case {
                name: "/saves path in old ∩ target, missing + staged, Repair(no re-enable) -> Keep/InSaveFolder (never resurrect)",
                old: Some(&[(SAVE_PATH, 1)]),
                target: &[(SAVE_PATH, 2)],
                staged: &[SAVE_PATH],
                disk: &[],
                mode: ApplyMode::Repair {
                    re_enable_disabled: false,
                },
                expect: Expect::Entry(PlanAction::Keep, PlanReason::InSaveFolder),
            },
            SAVE_PATH,
        );
    }

    #[test]
    fn saves_in_old_and_target_missing_staged_repair_reenable_keeps_in_save_folder() {
        run_for_path(
            Case {
                name: "/saves path in old ∩ target, missing + staged, Repair(re-enable) -> Keep/InSaveFolder (never resurrect)",
                old: Some(&[(SAVE_PATH, 1)]),
                target: &[(SAVE_PATH, 2)],
                staged: &[SAVE_PATH],
                disk: &[],
                mode: ApplyMode::Repair {
                    re_enable_disabled: true,
                },
                expect: Expect::Entry(PlanAction::Keep, PlanReason::InSaveFolder),
            },
            SAVE_PATH,
        );
    }

    // --- saves folder, target-only membership, disk already has bytes -----
    //
    // Closes a gap the pre-existing saves coverage left open: both
    // `saves_folder_is_always_kept` (in both old and target, or dropped —
    // only in old) and `repair_saves_folder_kept_even_when_damaged` (in
    // both old and target) only ever exercise paths `old` already knows
    // about. A `/saves` path that exists *only* in `target` — no prior
    // record at all, e.g. the first pack version to ship this world — must
    // still protect whatever is already sitting on disk, in every mode,
    // regardless of hash: Present-pristine (matches target's own hash),
    // Present-modified (an arbitrary unrelated hash — there is no `old`
    // hash for this path to be "pristine" against), and Disabled all keep.

    #[test]
    fn saves_target_only_present_or_disabled_always_protected() {
        let staged: HashSet<String> = HashSet::new();
        let target = packinfo(&[(SAVE_PATH, 2)]);

        let disk_states = [
            DiskState::Present { md5: hashes(2).md5 }, // pristine vs. target
            DiskState::Present { md5: hashes(9).md5 }, // modified vs. target
            DiskState::Disabled { md5: hashes(9).md5 },
        ];

        for disk_state in disk_states {
            let mut disk = HashMap::new();
            disk.insert(SAVE_PATH.to_string(), disk_state);

            for mode in [
                ApplyMode::VersionChange,
                ApplyMode::Repair {
                    re_enable_disabled: false,
                },
                ApplyMode::Repair {
                    re_enable_disabled: true,
                },
            ] {
                let entries = plan(PlanInputs {
                    old: None,
                    target: &target,
                    staged: &staged,
                    disk: &disk,
                    mode,
                })
                .unwrap_or_else(|e| panic!("target-only saves entries must never error, got {e}"));
                let entry = entries
                    .iter()
                    .find(|e| e.path == SAVE_PATH)
                    .expect("saves entry must be present (target-only)");
                assert_eq!(entry.action, PlanAction::Keep);
                assert_eq!(entry.reason, PlanReason::InSaveFolder);
            }
        }
    }

    // --- Repair, re_enable_disabled: true ---------------------------------

    #[test]
    fn repair_reenable_pristine_unstaged_reenables() {
        run(Case {
            name: "repair(re-enable), disabled twin pristine + unstaged -> ReEnable/ReEnabled (no error)",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Disabled(2))],
            mode: ApplyMode::Repair {
                re_enable_disabled: true,
            },
            expect: Expect::Entry(PlanAction::ReEnable, PlanReason::ReEnabled),
        });
    }

    #[test]
    fn repair_reenable_damaged_staged_reenables() {
        run(Case {
            name: "repair(re-enable), disabled twin damaged + staged -> ReEnable/ReEnabled",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Disabled(9))],
            mode: ApplyMode::Repair {
                re_enable_disabled: true,
            },
            expect: Expect::Entry(PlanAction::ReEnable, PlanReason::ReEnabled),
        });
    }

    #[test]
    fn repair_reenable_damaged_unstaged_errors() {
        run(Case {
            name: "repair(re-enable), disabled twin damaged + unstaged -> Error (modified twin needs pack bytes)",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Disabled(9))],
            mode: ApplyMode::Repair {
                re_enable_disabled: true,
            },
            expect: Expect::Error,
        });
    }

    // present/missing rows are identical to `re_enable_disabled: false`
    // (that flag only ever affects the Disabled arm) — mirror the five
    // target-present cases above to pin that down as a regression guard.

    #[test]
    fn repair_reenable_true_pristine_keeps_unchanged() {
        run(Case {
            name: "repair(re-enable=true), disk matches target -> Keep/Unchanged",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(2))],
            mode: ApplyMode::Repair {
                re_enable_disabled: true,
            },
            expect: Expect::Entry(PlanAction::Keep, PlanReason::Unchanged),
        });
    }

    #[test]
    fn repair_reenable_true_damaged_staged_replaces_overwrote() {
        run(Case {
            name: "repair(re-enable=true), disk damaged + staged -> Replace/RepairOverwrote",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Present(9))],
            mode: ApplyMode::Repair {
                re_enable_disabled: true,
            },
            expect: Expect::Entry(
                PlanAction::Replace,
                PlanReason::RepairOverwrote {
                    original: hashes(2).md5,
                    current: hashes(9).md5,
                },
            ),
        });
    }

    #[test]
    fn repair_reenable_true_damaged_unstaged_errors() {
        run(Case {
            name: "repair(re-enable=true), disk damaged + unstaged -> Error",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Present(9))],
            mode: ApplyMode::Repair {
                re_enable_disabled: true,
            },
            expect: Expect::Error,
        });
    }

    #[test]
    fn repair_reenable_true_missing_staged_creates_restored() {
        run(Case {
            name: "repair(re-enable=true), disk missing + staged -> Create/RepairRestored",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[PATH],
            disk: &[(PATH, DiskCase::Missing)],
            mode: ApplyMode::Repair {
                re_enable_disabled: true,
            },
            expect: Expect::Entry(PlanAction::Create, PlanReason::RepairRestored),
        });
    }

    #[test]
    fn repair_reenable_true_missing_unstaged_errors() {
        run(Case {
            name: "repair(re-enable=true), disk missing + unstaged -> Error",
            old: Some(&[(PATH, 5)]),
            target: &[(PATH, 2)],
            staged: &[],
            disk: &[(PATH, DiskCase::Missing)],
            mode: ApplyMode::Repair {
                re_enable_disabled: true,
            },
            expect: Expect::Error,
        });
    }

    // --- never-both-files property, non-vacuous multi-path case ----------
    //
    // Every `Case` above only ever varies a single path, so the duplicate
    // check embedded in `run()` (see above) is satisfied somewhat by
    // construction there. This test drives one `plan()` call over a richer
    // multi-path universe spanning several Repair buckets simultaneously
    // (including a path shared by both `old` and `target`, which is the
    // shape a dedup regression would actually slip through on).

    #[test]
    fn repair_output_has_at_most_one_entry_per_path() {
        const PRISTINE: &str = "/mods/pristine.jar";
        const DAMAGED: &str = "/mods/damaged.jar";
        const NEW: &str = "/mods/new.jar";
        const DROPPED: &str = "/mods/dropped.jar";
        const DISABLED: &str = "/mods/disabled.jar";
        const SAVE: &str = "/saves/world/level.dat";

        let old = packinfo(&[
            (PRISTINE, 5),
            (DAMAGED, 1),
            (DROPPED, 1),
            (DISABLED, 1),
            (SAVE, 1),
        ]);
        let target = packinfo(&[
            (PRISTINE, 2),
            (DAMAGED, 2),
            (NEW, 3),
            (DISABLED, 2),
            (SAVE, 1),
        ]);
        let staged: HashSet<String> = [DAMAGED, NEW, DISABLED]
            .into_iter()
            .map(String::from)
            .collect();

        let mut disk: HashMap<String, DiskState> = HashMap::new();
        disk.insert(
            PRISTINE.to_string(),
            DiskState::Present { md5: hashes(2).md5 },
        );
        disk.insert(
            DAMAGED.to_string(),
            DiskState::Present { md5: hashes(9).md5 },
        );
        // NEW absent from disk -> defaults to Missing.
        disk.insert(
            DROPPED.to_string(),
            DiskState::Present { md5: hashes(1).md5 },
        );
        disk.insert(
            DISABLED.to_string(),
            DiskState::Disabled { md5: hashes(9).md5 },
        );
        disk.insert(SAVE.to_string(), DiskState::Present { md5: hashes(1).md5 });

        let entries = plan(PlanInputs {
            old: Some(&old),
            target: &target,
            staged: &staged,
            disk: &disk,
            mode: ApplyMode::Repair {
                re_enable_disabled: true,
            },
        })
        .expect("all required staged sources present, must not error");

        let expected_paths: HashSet<String> = [PRISTINE, DAMAGED, NEW, DROPPED, DISABLED, SAVE]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(
            entries.len(),
            expected_paths.len(),
            "expected exactly one entry per path, got {entries:?}"
        );

        let mut seen = HashSet::new();
        for entry in &entries {
            assert!(
                seen.insert(entry.path.clone()),
                "duplicate plan entry for path {}, got {entries:?}",
                entry.path
            );
        }
        assert_eq!(seen, expected_paths);
    }
}
