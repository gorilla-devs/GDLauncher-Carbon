//! Carries a player's choice about a mod over to the file a pack update
//! ships that mod in next.
//!
//! The planner ([`super::apply_plan`]) knows files by path alone, and a mod's
//! file name carries its version. To the planner an update that ships a mod's
//! next version is one file dropped and an unrelated file added. Left at
//! that, a mod the player switched off comes back on, and a mod the player
//! replaced with their own copy gets the pack's copy beside it, which most
//! mod loaders refuse to start with.
//!
//! Here two files are the same mod when their jars declare the same mod id
//! and sit in the same folder under `/mods`. That is used in two steps, and
//! by a version change only: both go by plan entries that Repair never
//! makes, as it restores the pack as it is (its preview lists duplicates
//! itself).
//!
//! - [`carry`] runs between planning and applying. A file the update adds is
//!   installed switched off when the player switched off the file of that mod
//!   the update drops, and is left out when the player's own file of that mod
//!   is in place.
//! - [`Carried::replaced`] is asked once everything is published. It names
//!   the switched-off files the update dropped that can go: still the pack's
//!   original, with a successor in place.
//!
//! Nothing is remembered between attempts. Both steps go by what is on disk,
//! so an interrupted apply decides the same way again. A jar that declares no
//! id is the same mod as nothing and is handled by path, as before. So is a
//! mod the player deleted, which leaves no jar to read an id from.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};

use super::apply_plan::{PlanAction, PlanEntry, PlanReason};
use super::disk_scan;
use super::packinfo::PackInfo;

/// What [`carry`] decided, for the apply to act on and its audit to tell.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Carried {
    /// Files to install switched off, each with the switched-off file of the
    /// same mod that the update drops.
    switched_off: BTreeMap<String, String>,
    /// Files left out, each with the player's own file of the same mod.
    left_out: BTreeMap<String, String>,
    /// Switched-off files the update drops, to remove once a successor is in
    /// place.
    replaced: Vec<Replaced>,
}

#[derive(Debug, PartialEq, Eq)]
struct Replaced {
    path: String,
    /// The md5 the pack recorded for it: only the pack's original goes.
    original: [u8; 16],
    /// The files of the same mod the update adds, each with the md5 it is
    /// installed with.
    successors: Vec<(String, [u8; 16])>,
}

impl Carried {
    /// Whether the apply creates `path` under its `.disabled` name.
    pub fn lands_switched_off(&self, path: &str) -> bool {
        self.switched_off.contains_key(path)
    }

    /// The switched-off files that can go now, by plan path and by the file
    /// to remove: each still holds the bytes the pack shipped, and a file of
    /// the same mod from the new version is in place, switched off, with the
    /// pack's bytes. Asked after the apply has published, so it holds for a
    /// first attempt and for one that resumes after the successor landed.
    pub async fn replaced(&self, data: &Path) -> Vec<(String, PathBuf)> {
        let mut ready = Vec::new();
        for replaced in &self.replaced {
            let file = disabled_name(&live(data, &replaced.path));
            if md5_of(&file).await != Some(replaced.original) {
                continue;
            }
            for (successor, md5) in &replaced.successors {
                if md5_of(&disabled_name(&live(data, successor))).await == Some(*md5) {
                    ready.push((replaced.path.clone(), file));
                    break;
                }
            }
        }
        ready
    }

    /// Lines for the audit: what the apply did because of a file of the same
    /// mod. `outcomes` is what the apply made of the plan.
    pub fn notes(&self, outcomes: &[PlanEntry]) -> Vec<String> {
        let created: HashSet<&str> = outcomes
            .iter()
            .filter(|entry| entry.action == PlanAction::Create)
            .map(|entry| entry.path.as_str())
            .collect();
        let switched_off = self
            .switched_off
            .iter()
            .filter(|(path, _)| created.contains(path.as_str()))
            .map(|(path, previous)| {
                format!(
                    "{path} was installed switched off: it is the same mod as {previous}, which \
                     was switched off."
                )
            });
        let left_out = self.left_out.iter().map(|(path, theirs)| {
            format!("{path} was not installed: {theirs} is the same mod and was kept.")
        });
        switched_off.chain(left_out).collect()
    }
}

/// Adjusts the plan of a version change for the files that are the same mod
/// as one the player made a choice about, and returns what the apply needs
/// to finish it. `data` is the instance's data folder and `staged` the
/// staged copy of it that the plan installs from.
///
/// Reads jars only when a folder under `/mods` gets a new file while it
/// holds a switched-off pack file the update drops, or a jar of the
/// player's. A jar that can't be read declares no id.
pub async fn carry(
    entries: &mut [PlanEntry],
    old: &PackInfo,
    target: &PackInfo,
    data: &Path,
    staged: &Path,
    fs_case_insensitive: bool,
) -> Carried {
    let roles = Roles::of(entries, old, target);
    if roles.added.is_empty() && roles.landed.is_empty() {
        return Carried::default();
    }
    let tracked: HashSet<String> = old
        .files
        .keys()
        .chain(target.files.keys())
        .map(|path| fold(path, fs_case_insensitive))
        .collect();
    let (data, staged) = (data.to_owned(), staged.to_owned());
    let read = tokio::task::spawn_blocking(move || {
        read_jars(&roles, &tracked, &data, &staged, fs_case_insensitive)
    })
    .await;
    match read {
        Ok((own, ids)) => decide(entries, old, target, &own, &ids),
        Err(error) => {
            tracing::warn!("Could not read the mods of this update: {error}");
            Carried::default()
        }
    }
}

/// The jars of one apply that can be the same mod as another, by what the
/// plan does with each. Only jars in a folder under `/mods`.
#[derive(Debug, Default)]
struct Roles {
    /// Added by the update, and created by this apply.
    added: Vec<String>,
    /// Added by the update and found in place: an earlier attempt of this
    /// apply may have installed it.
    landed: Vec<String>,
    /// Dropped by the update, and switched off by the player.
    switched_off: Vec<String>,
    /// Dropped by the update, and kept as it holds the player's bytes.
    changed: Vec<String>,
}

impl Roles {
    fn of(entries: &[PlanEntry], old: &PackInfo, target: &PackInfo) -> Self {
        let mut roles = Self::default();
        for entry in entries {
            if mods_folder(&entry.path).is_none() {
                continue;
            }
            let dropped = old.files.contains_key(&entry.path);
            let shipped = target.files.contains_key(&entry.path);
            let role = match (dropped, shipped, &entry.action, &entry.reason) {
                (false, true, PlanAction::Create, PlanReason::PackUpdate) => &mut roles.added,
                (false, true, PlanAction::Keep, PlanReason::PreservedExisting) => &mut roles.landed,
                (true, false, PlanAction::Keep, PlanReason::DisabledByUser) => {
                    &mut roles.switched_off
                }
                (true, false, PlanAction::Keep, PlanReason::DroppedButModified { .. }) => {
                    &mut roles.changed
                }
                _ => continue,
            };
            role.push(entry.path.clone());
        }
        roles
    }
}

/// The pure half of [`carry`]. `own` are the enabled jars in the instance
/// that neither pack version ships, as plan paths, and `ids` the mod id of
/// every jar that was read, by plan path.
fn decide(
    entries: &mut [PlanEntry],
    old: &PackInfo,
    target: &PackInfo,
    own: &BTreeSet<String>,
    ids: &HashMap<String, String>,
) -> Carried {
    let roles = Roles::of(entries, old, target);
    let same_mod = |a: &str, b: &str| {
        mods_folder(a) == mods_folder(b) && ids.get(a).is_some() && ids.get(a) == ids.get(b)
    };
    let mut carried = Carried::default();
    for added in &roles.added {
        // The player's own file comes first: with it in place the mod runs,
        // whatever they did with the pack's earlier file of it.
        let theirs = own
            .iter()
            .chain(&roles.changed)
            .find(|file| same_mod(added, file));
        if let Some(theirs) = theirs {
            carried.left_out.insert(added.clone(), theirs.clone());
        } else if let Some(previous) = roles.switched_off.iter().find(|f| same_mod(added, f)) {
            carried.switched_off.insert(added.clone(), previous.clone());
        }
    }
    for previous in &roles.switched_off {
        let successors: Vec<_> = roles
            .added
            .iter()
            .chain(&roles.landed)
            .filter(|file| same_mod(previous, file) && !carried.left_out.contains_key(*file))
            .map(|file| (file.clone(), target.files[file].md5))
            .collect();
        if !successors.is_empty() {
            carried.replaced.push(Replaced {
                path: previous.clone(),
                original: old.files[previous].md5,
                successors,
            });
        }
    }
    for entry in entries {
        if carried.left_out.contains_key(&entry.path) {
            entry.action = PlanAction::Keep;
            entry.reason = PlanReason::PreservedExisting;
        }
    }
    carried
}

/// The reading half of [`carry`]: the player's own enabled jars in the
/// folders that get a new file, and the mod id of every jar [`decide`] may
/// compare. The files the update adds are read only for a folder that holds
/// a jar they could be the same mod as.
fn read_jars(
    roles: &Roles,
    tracked: &HashSet<String>,
    data: &Path,
    staged: &Path,
    fs_case_insensitive: bool,
) -> (BTreeSet<String>, HashMap<String, String>) {
    let folders: BTreeSet<&str> = roles
        .added
        .iter()
        .chain(&roles.landed)
        .filter_map(|path| mods_folder(path))
        .collect();
    let mut own = BTreeSet::new();
    for folder in &folders {
        let Ok(files) = std::fs::read_dir(live(data, folder)) else {
            continue;
        };
        for file in files.flatten() {
            let Some(name) = file.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let path = format!("{folder}/{name}");
            if mods_folder(&path).is_some()
                && !tracked.contains(&fold(&path, fs_case_insensitive))
                && file.path().is_file()
            {
                own.insert(path);
            }
        }
    }

    let mut ids = HashMap::new();
    let mut read = |ids: &mut HashMap<String, String>, path: &String, jar: PathBuf| {
        if let Some(id) = mod_id(&jar) {
            ids.insert(path.clone(), id);
        }
    };
    for path in own.iter().chain(&roles.changed) {
        read(&mut ids, path, live(data, path));
    }
    for path in &roles.switched_off {
        read(&mut ids, path, disabled_name(&live(data, path)));
    }
    let held: HashSet<String> = ids
        .keys()
        .filter_map(|path| mods_folder(path))
        .map(str::to_owned)
        .collect();
    let in_held = |path: &str| mods_folder(path).is_some_and(|folder| held.contains(folder));
    for path in roles.added.iter().filter(|path| in_held(path)) {
        let bare = live(staged, path);
        let jar = if bare.is_file() {
            bare
        } else {
            disabled_name(&bare)
        };
        read(&mut ids, path, jar);
    }
    for path in roles.landed.iter().filter(|path| in_held(path)) {
        read(&mut ids, path, disabled_name(&live(data, path)));
    }
    (own, ids)
}

/// The mod id `jar` declares, read as the mods list reads it.
fn mod_id(jar: &Path) -> Option<String> {
    let file = std::fs::File::open(jar).ok()?;
    let metadata =
        crate::managers::metadata::mods::parse_metadata(&mut std::io::BufReader::new(file))
            .ok()??;
    metadata.modid.filter(|id| !id.is_empty())
}

/// The folder of `path` when it is a jar somewhere under `/mods`.
fn mods_folder(path: &str) -> Option<&str> {
    let (folder, name) = path.rsplit_once('/')?;
    let name = name.as_bytes();
    let jar = name.len() > 4 && name[name.len() - 4..].eq_ignore_ascii_case(b".jar");
    (jar && (folder == "/mods" || folder.starts_with("/mods/"))).then_some(folder)
}

fn fold(path: &str, fs_case_insensitive: bool) -> String {
    if fs_case_insensitive {
        path.to_ascii_lowercase()
    } else {
        path.to_owned()
    }
}

/// Where a plan path is in `folder`. Plan paths start with `/` and were
/// checked by the disk scan before any plan was made from them.
fn live(folder: &Path, path: &str) -> PathBuf {
    folder.join(path.trim_start_matches('/'))
}

fn disabled_name(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".disabled");
    PathBuf::from(name)
}

/// The md5 of `path`, or `None` when it is not there or can't be read.
async fn md5_of(path: &Path) -> Option<[u8; 16]> {
    disk_scan::probe_md5(path).await.ok().flatten()
}

/// A jar that declares `id`, for tests here and of the apply. `version` only
/// makes the bytes of two jars of one mod differ.
#[cfg(test)]
pub(crate) fn test_jar(id: &str, version: &str) -> Vec<u8> {
    use std::io::Write;
    let mut bytes = Vec::new();
    let mut zip = zip::ZipWriter::new(std::io::Cursor::new(&mut bytes));
    let options =
        zip::write::FileOptions::<()>::default().compression_method(zip::CompressionMethod::Stored);
    zip.start_file("fabric.mod.json", options).unwrap();
    write!(
        zip,
        r#"{{"schemaVersion":1,"id":"{id}","version":"{version}"}}"#
    )
    .unwrap();
    zip.finish().unwrap();
    bytes
}

#[cfg(test)]
mod tests {
    use super::super::apply_plan::{self, ApplyMode, DiskState, PlanInputs};
    use super::super::packinfo::FileHashes;
    use super::*;

    const OLD: u8 = 1;
    const NEW: u8 = 2;

    fn pack(paths: &[&str], seed: u8) -> PackInfo {
        PackInfo {
            files: paths
                .iter()
                .map(|path| {
                    let hashes = FileHashes {
                        sha512: [seed; 64],
                        md5: [seed; 16],
                    };
                    (path.to_string(), hashes)
                })
                .collect(),
        }
    }

    fn ids(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(path, id)| (path.to_string(), id.to_string()))
            .collect()
    }

    fn own(paths: &[&str]) -> BTreeSet<String> {
        paths.iter().map(|path| path.to_string()).collect()
    }

    /// The plan of a version change from `old` to `target`, every target
    /// file staged.
    fn plan(old: &PackInfo, target: &PackInfo, disk: &[(&str, DiskState)]) -> Vec<PlanEntry> {
        let disk: HashMap<String, DiskState> = disk
            .iter()
            .map(|(path, state)| (path.to_string(), *state))
            .collect();
        apply_plan::plan(PlanInputs {
            old: Some(old),
            target,
            staged: &target.files.keys().cloned().collect(),
            disk: &disk,
            coexisting_disabled_twin_md5: &HashMap::new(),
            mode: ApplyMode::VersionChange,
            fs_case_insensitive: false,
        })
        .unwrap()
    }

    fn entry<'a>(entries: &'a [PlanEntry], path: &str) -> (&'a PlanAction, &'a PlanReason) {
        let entry = entries.iter().find(|entry| entry.path == path).unwrap();
        (&entry.action, &entry.reason)
    }

    const OFF: DiskState = DiskState::Disabled { md5: [OLD; 16] };

    #[test]
    fn the_next_version_of_a_switched_off_mod_is_installed_switched_off() {
        let old = pack(&["/mods/map-1.0.jar", "/mods/other-1.0.jar"], OLD);
        let target = pack(&["/mods/map-1.1.jar", "/mods/other-1.1.jar"], NEW);
        let present = DiskState::Present { md5: [OLD; 16] };
        let mut entries = plan(
            &old,
            &target,
            &[("/mods/map-1.0.jar", OFF), ("/mods/other-1.0.jar", present)],
        );
        let before = entries.clone();

        let carried = decide(
            &mut entries,
            &old,
            &target,
            &own(&[]),
            &ids(&[
                ("/mods/map-1.0.jar", "map"),
                ("/mods/map-1.1.jar", "map"),
                ("/mods/other-1.1.jar", "other"),
            ]),
        );

        assert!(carried.lands_switched_off("/mods/map-1.1.jar"));
        assert!(!carried.lands_switched_off("/mods/other-1.1.jar"));
        assert_eq!(
            carried.replaced,
            [Replaced {
                path: "/mods/map-1.0.jar".into(),
                original: [OLD; 16],
                successors: vec![("/mods/map-1.1.jar".into(), [NEW; 16])],
            }]
        );
        assert_eq!(entries, before, "the plan itself still creates the file");
    }

    #[test]
    fn the_packs_copy_is_left_out_beside_the_players_own_copy_of_the_mod() {
        let old = pack(&["/mods/jei-1.0.jar"], OLD);
        let target = pack(&["/mods/jei-1.1.jar", "/mods/new-1.0.jar"], NEW);
        let mut entries = plan(&old, &target, &[]);

        let carried = decide(
            &mut entries,
            &old,
            &target,
            &own(&["/mods/jei-1.2.jar"]),
            &ids(&[
                ("/mods/jei-1.2.jar", "jei"),
                ("/mods/jei-1.1.jar", "jei"),
                ("/mods/new-1.0.jar", "new"),
            ]),
        );

        assert_eq!(
            entry(&entries, "/mods/jei-1.1.jar"),
            (&PlanAction::Keep, &PlanReason::PreservedExisting)
        );
        assert_eq!(
            entry(&entries, "/mods/new-1.0.jar"),
            (&PlanAction::Create, &PlanReason::PackUpdate)
        );
        assert_eq!(
            carried.notes(&entries),
            [
                "/mods/jei-1.1.jar was not installed: /mods/jei-1.2.jar is the same mod and was kept."
            ]
        );
        assert!(!carried.lands_switched_off("/mods/jei-1.1.jar"));
    }

    #[test]
    fn a_pack_file_the_player_changed_in_place_counts_as_their_own_copy() {
        let old = pack(&["/mods/jei.jar"], OLD);
        let target = pack(&["/mods/jei-1.1.jar"], NEW);
        let changed = DiskState::Present { md5: [9; 16] };
        let mut entries = plan(&old, &target, &[("/mods/jei.jar", changed)]);

        decide(
            &mut entries,
            &old,
            &target,
            &own(&[]),
            &ids(&[("/mods/jei.jar", "jei"), ("/mods/jei-1.1.jar", "jei")]),
        );

        assert_eq!(
            entry(&entries, "/mods/jei-1.1.jar"),
            (&PlanAction::Keep, &PlanReason::PreservedExisting)
        );
    }

    #[test]
    fn the_players_own_copy_decides_before_the_file_they_switched_off() {
        let old = pack(&["/mods/jei-1.0.jar"], OLD);
        let target = pack(&["/mods/jei-1.1.jar"], NEW);
        let mut entries = plan(&old, &target, &[("/mods/jei-1.0.jar", OFF)]);

        let carried = decide(
            &mut entries,
            &old,
            &target,
            &own(&["/mods/jei-1.2.jar"]),
            &ids(&[
                ("/mods/jei-1.0.jar", "jei"),
                ("/mods/jei-1.1.jar", "jei"),
                ("/mods/jei-1.2.jar", "jei"),
            ]),
        );

        assert_eq!(
            entry(&entries, "/mods/jei-1.1.jar"),
            (&PlanAction::Keep, &PlanReason::PreservedExisting)
        );
        assert!(!carried.lands_switched_off("/mods/jei-1.1.jar"));
        assert!(
            carried.replaced.is_empty(),
            "nothing takes the place of the file they switched off"
        );
    }

    #[test]
    fn files_are_the_same_mod_only_with_the_same_id_in_the_same_folder() {
        let old = pack(&["/mods/map-1.0.jar"], OLD);
        let target = pack(
            &[
                "/mods/another-id.jar",
                "/mods/no-id.jar",
                "/mods/sub/another-folder.jar",
            ],
            NEW,
        );
        let mut entries = plan(&old, &target, &[("/mods/map-1.0.jar", OFF)]);
        let before = entries.clone();

        let carried = decide(
            &mut entries,
            &old,
            &target,
            &own(&["/mods/sub/own.jar", "/mods/own-without-id.jar"]),
            &ids(&[
                ("/mods/map-1.0.jar", "map"),
                ("/mods/another-id.jar", "minimap"),
                ("/mods/sub/another-folder.jar", "map"),
                ("/mods/sub/own.jar", "minimap"),
            ]),
        );

        assert_eq!(carried, Carried::default());
        assert_eq!(entries, before);
    }

    #[test]
    fn only_jars_under_the_mods_folder_are_compared() {
        for folder in ["/config", "/resourcepacks", "/modsbackup"] {
            let dropped = format!("{folder}/a-1.0.jar");
            let added = format!("{folder}/a-1.1.jar");
            let old = pack(&[&dropped], OLD);
            let target = pack(&[&added], NEW);
            let mut entries = plan(&old, &target, &[(&dropped, OFF)]);

            let carried = decide(
                &mut entries,
                &old,
                &target,
                &own(&[]),
                &ids(&[(&dropped, "a"), (&added, "a")]),
            );

            assert_eq!(carried, Carried::default(), "{folder}");
        }
        assert_eq!(mods_folder("/mods/a.JAR"), Some("/mods"));
        assert_eq!(mods_folder("/mods/1.20/a.jar"), Some("/mods/1.20"));
        assert_eq!(mods_folder("/mods/a.jar.disabled"), None);
        assert_eq!(mods_folder("/mods/a.zip"), None);
        assert_eq!(mods_folder("/mods/.jar"), None);
    }

    #[test]
    fn two_files_of_one_mod_that_the_pack_itself_ships_are_both_installed() {
        let old = pack(&["/mods/lib-1.0.jar"], OLD);
        let target = pack(&["/mods/lib-1.0.jar", "/mods/lib-extra.jar"], OLD);
        let present = DiskState::Present { md5: [OLD; 16] };
        let mut entries = plan(&old, &target, &[("/mods/lib-1.0.jar", present)]);
        let before = entries.clone();

        let carried = decide(
            &mut entries,
            &old,
            &target,
            &own(&[]),
            &ids(&[("/mods/lib-1.0.jar", "lib"), ("/mods/lib-extra.jar", "lib")]),
        );

        assert_eq!(carried, Carried::default());
        assert_eq!(entries, before);
    }

    #[test]
    fn a_successor_an_earlier_attempt_installed_still_replaces_the_old_file() {
        let old = pack(&["/mods/map-1.0.jar"], OLD);
        let target = pack(&["/mods/map-1.1.jar"], NEW);
        let landed = DiskState::Disabled { md5: [NEW; 16] };
        let mut entries = plan(
            &old,
            &target,
            &[("/mods/map-1.0.jar", OFF), ("/mods/map-1.1.jar", landed)],
        );

        let carried = decide(
            &mut entries,
            &old,
            &target,
            &own(&[]),
            &ids(&[("/mods/map-1.0.jar", "map"), ("/mods/map-1.1.jar", "map")]),
        );

        assert_eq!(carried.replaced.len(), 1);
        assert_eq!(
            carried.replaced[0].successors,
            [("/mods/map-1.1.jar".to_string(), [NEW; 16])]
        );
        assert!(carried.notes(&entries).is_empty(), "nothing was created");
    }

    #[test]
    fn a_repair_is_left_as_planned() {
        let old = pack(&["/mods/map-1.0.jar"], OLD);
        let target = pack(&["/mods/map-1.1.jar", "/mods/jei-1.1.jar"], NEW);
        let disk = HashMap::from([("/mods/map-1.0.jar".to_string(), OFF)]);
        let mut entries = apply_plan::plan(PlanInputs {
            old: Some(&old),
            target: &target,
            staged: &target.files.keys().cloned().collect(),
            disk: &disk,
            coexisting_disabled_twin_md5: &HashMap::new(),
            mode: ApplyMode::Repair {
                re_enable_disabled: false,
            },
            fs_case_insensitive: false,
        })
        .unwrap();
        let before = entries.clone();

        let carried = decide(
            &mut entries,
            &old,
            &target,
            &own(&["/mods/jei-1.2.jar"]),
            &ids(&[
                ("/mods/map-1.0.jar", "map"),
                ("/mods/map-1.1.jar", "map"),
                ("/mods/jei-1.1.jar", "jei"),
                ("/mods/jei-1.2.jar", "jei"),
            ]),
        );

        assert_eq!(carried, Carried::default());
        assert_eq!(entries, before);
    }

    #[test]
    fn the_audit_tells_only_what_the_apply_did() {
        let old = pack(&["/mods/map-1.0.jar"], OLD);
        let target = pack(&["/mods/map-1.1.jar"], NEW);
        let mut entries = plan(&old, &target, &[("/mods/map-1.0.jar", OFF)]);
        let carried = decide(
            &mut entries,
            &old,
            &target,
            &own(&[]),
            &ids(&[("/mods/map-1.0.jar", "map"), ("/mods/map-1.1.jar", "map")]),
        );

        assert_eq!(
            carried.notes(&entries),
            [
                "/mods/map-1.1.jar was installed switched off: it is the same mod as \
              /mods/map-1.0.jar, which was switched off."
            ]
        );

        // The apply found the name taken and installed nothing there.
        for entry in &mut entries {
            entry.action = PlanAction::Keep;
            entry.reason = PlanReason::PreservedExisting;
        }
        assert!(carried.notes(&entries).is_empty());
    }

    /// A data folder and its staged copy, on disk.
    struct Folders {
        _keep: tempfile::TempDir,
        data: PathBuf,
        staged: PathBuf,
    }

    impl Folders {
        fn new() -> Self {
            let keep = tempfile::tempdir().unwrap();
            let (data, staged) = (keep.path().join("instance"), keep.path().join("staged"));
            std::fs::create_dir_all(data.join("mods")).unwrap();
            std::fs::create_dir_all(staged.join("mods")).unwrap();
            Self {
                _keep: keep,
                data,
                staged,
            }
        }

        fn write(&self, folder: &Path, path: &str, bytes: &[u8]) {
            let file = folder.join(path.trim_start_matches('/'));
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }

        /// The pack record of what is in `folder` right now.
        async fn pack(&self, folder: &Path) -> PackInfo {
            super::super::packinfo::scan_dir(folder, None)
                .await
                .unwrap()
        }

        /// Plans the version change as the apply does, from a scan of the
        /// data folder, and carries the player's choices into it.
        async fn carry(&self, old: &PackInfo, target: &PackInfo) -> (Vec<PlanEntry>, Carried) {
            let universe = old.files.keys().chain(target.files.keys()).cloned();
            let scan = disk_scan::scan_disk_state(&self.data, &universe.collect())
                .await
                .unwrap();
            let mut entries = apply_plan::plan(PlanInputs {
                old: Some(old),
                target,
                staged: &target.files.keys().cloned().collect(),
                disk: &scan.states,
                coexisting_disabled_twin_md5: &scan.coexisting_disabled_twin_md5,
                mode: ApplyMode::VersionChange,
                fs_case_insensitive: false,
            })
            .unwrap();
            let carried = carry(&mut entries, old, target, &self.data, &self.staged, false).await;
            (entries, carried)
        }
    }

    #[tokio::test]
    async fn ids_are_read_from_the_switched_off_jar_and_the_staged_one() {
        let folders = Folders::new();
        folders.write(&folders.data, "/mods/map-1.0.jar", &test_jar("map", "1.0"));
        let old = folders.pack(&folders.data).await;
        std::fs::rename(
            folders.data.join("mods/map-1.0.jar"),
            folders.data.join("mods/map-1.0.jar.disabled"),
        )
        .unwrap();
        folders.write(
            &folders.staged,
            "/mods/map-1.1.jar",
            &test_jar("map", "1.1"),
        );
        folders.write(&folders.staged, "/mods/other.jar", &test_jar("other", "1"));
        folders.write(&folders.staged, "/mods/not-a-jar.jar", b"plain text");
        let target = folders.pack(&folders.staged).await;

        let (entries, carried) = folders.carry(&old, &target).await;

        assert!(carried.lands_switched_off("/mods/map-1.1.jar"));
        assert!(!carried.lands_switched_off("/mods/other.jar"));
        assert!(!carried.lands_switched_off("/mods/not-a-jar.jar"));
        assert!(
            carried.replaced(&folders.data).await.is_empty(),
            "the old file stays until its successor is in place"
        );
        assert_eq!(
            entry(&entries, "/mods/map-1.1.jar"),
            (&PlanAction::Create, &PlanReason::PackUpdate)
        );
    }

    #[tokio::test]
    async fn a_pack_shipped_disabled_successor_is_read_from_its_disabled_name() {
        let folders = Folders::new();
        folders.write(&folders.data, "/mods/map-1.0.jar", &test_jar("map", "1.0"));
        let old = folders.pack(&folders.data).await;
        std::fs::rename(
            folders.data.join("mods/map-1.0.jar"),
            folders.data.join("mods/map-1.0.jar.disabled"),
        )
        .unwrap();
        folders.write(
            &folders.staged,
            "/mods/map-1.1.jar.disabled",
            &test_jar("map", "1.1"),
        );
        let target = folders.pack(&folders.staged).await;

        let (_, carried) = folders.carry(&old, &target).await;

        assert!(carried.lands_switched_off("/mods/map-1.1.jar"));
    }

    #[tokio::test]
    async fn the_players_own_jars_are_the_enabled_ones_neither_version_ships() {
        let folders = Folders::new();
        folders.write(
            &folders.data,
            "/mods/kept-1.0.jar",
            &test_jar("kept", "1.0"),
        );
        let old = folders.pack(&folders.data).await;
        // The player's own: a newer jei, and a jar of theirs they switched
        // off, which the game does not load.
        folders.write(&folders.data, "/mods/jei-1.2.jar", &test_jar("jei", "1.2"));
        folders.write(
            &folders.data,
            "/mods/sodium-own.jar.disabled",
            &test_jar("sodium", "9"),
        );
        // The pack now ships a second file of a mod it already ships: both
        // are the pack's, neither is the player's.
        for (path, id) in [
            ("/mods/kept-1.0.jar", "kept"),
            ("/mods/kept-addon.jar", "kept"),
            ("/mods/jei-1.1.jar", "jei"),
            ("/mods/sodium-1.0.jar", "sodium"),
        ] {
            let version = if path == "/mods/kept-1.0.jar" {
                "1.0"
            } else {
                "pack"
            };
            folders.write(&folders.staged, path, &test_jar(id, version));
        }
        let target = folders.pack(&folders.staged).await;

        let (entries, carried) = folders.carry(&old, &target).await;

        assert_eq!(
            entry(&entries, "/mods/jei-1.1.jar"),
            (&PlanAction::Keep, &PlanReason::PreservedExisting)
        );
        for installed in ["/mods/sodium-1.0.jar", "/mods/kept-addon.jar"] {
            assert_eq!(
                entry(&entries, installed),
                (&PlanAction::Create, &PlanReason::PackUpdate),
                "{installed}"
            );
        }
        assert_eq!(
            carried.left_out,
            BTreeMap::from([(
                "/mods/jei-1.1.jar".to_string(),
                "/mods/jei-1.2.jar".to_string()
            )])
        );
    }

    #[tokio::test]
    async fn the_old_file_goes_only_as_the_packs_original_with_its_successor_in_place() {
        let folders = Folders::new();
        let (original, next) = (test_jar("map", "1.0"), test_jar("map", "1.1"));
        folders.write(&folders.data, "/mods/map-1.0.jar", &original);
        let old = folders.pack(&folders.data).await;
        let old_file = folders.data.join("mods/map-1.0.jar.disabled");
        std::fs::rename(folders.data.join("mods/map-1.0.jar"), &old_file).unwrap();
        folders.write(&folders.staged, "/mods/map-1.1.jar", &next);
        let target = folders.pack(&folders.staged).await;
        let (_, carried) = folders.carry(&old, &target).await;
        let ready = vec![("/mods/map-1.0.jar".to_string(), old_file.clone())];

        assert!(carried.replaced(&folders.data).await.is_empty());

        // Enabled instead of switched off: not what replaces it.
        folders.write(&folders.data, "/mods/map-1.1.jar", &next);
        assert!(carried.replaced(&folders.data).await.is_empty());
        std::fs::remove_file(folders.data.join("mods/map-1.1.jar")).unwrap();

        // Switched off, but not the pack's bytes.
        folders.write(
            &folders.data,
            "/mods/map-1.1.jar.disabled",
            &test_jar("map", "theirs"),
        );
        assert!(carried.replaced(&folders.data).await.is_empty());

        folders.write(&folders.data, "/mods/map-1.1.jar.disabled", &next);
        assert_eq!(carried.replaced(&folders.data).await, ready);

        // The player changed the old file: it is theirs to keep.
        std::fs::write(&old_file, test_jar("map", "edited")).unwrap();
        assert!(carried.replaced(&folders.data).await.is_empty());
    }
}
