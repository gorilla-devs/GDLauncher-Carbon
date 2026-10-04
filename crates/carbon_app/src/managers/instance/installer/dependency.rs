//! One rule for required dependencies, as in 2.0.40, Prism Launcher and the
//! Modrinth App:
//! - an enabled installed copy satisfies the requirement, whatever its version,
//!   loader tags or file name, and next to any disabled or duplicate copy; so
//!   does an enabled jar with the mod id of the project or of its disabled copy;
//! - a disabled-only copy counts as present: the addon is installed next to it
//!   and the copy stays disabled, as 2.0.40 did. A fresh install also warns
//!   that the dependency is disabled, naming it; an update says nothing;
//! - a missing dependency that cannot be resolved is left out with a warning
//!   naming it, unless only the parent's build for another loader needs it.
//!   Only the addon the user picked has to resolve;
//! - a Modrinth pin that is not for this game and loader is replaced by the
//!   same release built for it, else by the newest compatible version, with a
//!   warning naming both versions when they are different releases;
//! - when two dependants pin different versions of a missing dependency, only
//!   one can be installed: the newest of them, with a warning naming it (see
//!   [`ConflictingPin`]).
//!
//! Installed dependencies are never re-fetched, re-selected or re-validated,
//! and their own dependencies are never walked. Strict selection only applies
//! to what is actually going to be downloaded.
//!
//! An update only adds the requirements its new version introduces; the ones
//! the replaced version already declared are left as the user has them, as
//! every 2.0.40 update did (see [`Previous`]).
use super::*;
use crate::domain::instance::info::ModLoaderType;
use crate::domain::vtask::{DependencyPlatform, SkippedDependencyReason, TaskWarning};
use carbon_platforms::curseforge::CurseForgeResponse;
use std::collections::{HashMap, HashSet};

// ---------------------------------------------------------------------------
// Loader compatibility, shared with server addons.

/// Loader tags a file may carry to run on an instance or server with `loaders`
/// at `release`. Quilt also loads Fabric mods and NeoForge 1.20.1 also loads
/// Forge mods (Prism Launcher's getSupportedModLoaders).
pub(crate) fn accepted_loader_tags(
    release: &str,
    loaders: impl IntoIterator<Item = ModLoaderType>,
) -> Vec<&'static str> {
    let mut accepted = Vec::new();
    for loader in loaders {
        match loader {
            ModLoaderType::Forge => accepted.push("forge"),
            ModLoaderType::Neoforge => {
                accepted.push("neoforge");
                if release == "1.20.1" {
                    accepted.push("forge");
                }
            }
            ModLoaderType::Fabric => accepted.push("fabric"),
            ModLoaderType::Quilt => accepted.extend(["quilt", "fabric"]),
        }
    }
    accepted.sort_unstable();
    accepted.dedup();
    accepted
}

/// Every tag that names a mod loader or plugin platform. CurseForge mixes them
/// with game versions in `gameVersions`; Modrinth lists them in `loaders`.
const LOADER_TAGS: &[&str] = &[
    "forge",
    "neoforge",
    "fabric",
    "quilt",
    "liteloader",
    "cauldron",
    "rift",
    "modloader",
    "risugami's modloader",
    "babric",
    "bta-babric",
    "legacy-fabric",
    "ornithe",
    "nilloader",
    "java-agent",
    "bukkit",
    "spigot",
    "paper",
    "purpur",
    "folia",
    "sponge",
    "bungeecord",
    "velocity",
    "waterfall",
    "datapack",
    "minecraft",
    "iris",
    "optifine",
    "canvas",
    "vanilla",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoaderFit {
    /// The file lists one of the accepted loaders.
    Tagged,
    /// The file names no loader at all, as legacy CurseForge files do (most
    /// 1.12.2 libraries list only "1.12.2"). Treated as loader-agnostic.
    Untagged,
    /// The file is for other loaders only, or there is no loader to run it.
    Foreign,
}

pub(crate) fn loader_fit(tags: &[String], accepted: &[&str]) -> LoaderFit {
    // A vanilla target cannot load any mod, tagged or not.
    if accepted.is_empty() {
        return LoaderFit::Foreign;
    }
    let mut names_a_loader = false;
    for tag in tags {
        let tag = tag.to_lowercase();
        if accepted.contains(&tag.as_str()) {
            return LoaderFit::Tagged;
        }
        names_a_loader |= is_loader_tag(&tag);
    }
    if names_a_loader {
        LoaderFit::Foreign
    } else {
        LoaderFit::Untagged
    }
}

/// Whether a tag names a mod loader or plugin platform (see [`LOADER_TAGS`]).
pub(crate) fn is_loader_tag(tag: &str) -> bool {
    LOADER_TAGS.contains(&tag.to_lowercase().as_str())
}

/// The candidate a required dependency is downloaded at, from candidates
/// newest first (instances and servers): one built for the target's loaders
/// before one that names no loader, never one for other loaders only (see
/// [`loader_fit`]); among those, the newest at `preferred` stability or
/// better, else the newest of the most stable channel there is. `fit` is the
/// candidate's loader fit and channel, or `None` when it is not for the
/// target's game version.
pub(crate) fn best_candidate<'a, T>(
    candidates: &'a [T],
    preferred: ModChannel,
    fit: impl Fn(&T) -> Option<(LoaderFit, ModChannel)>,
) -> Option<&'a T> {
    let usable = |candidate: &T| fit(candidate).filter(|(fit, _)| *fit != LoaderFit::Foreign);
    // An untagged file is only a fallback when no file names our loader.
    let tagged = candidates
        .iter()
        .any(|candidate| matches!(usable(candidate), Some((LoaderFit::Tagged, _))));
    let mut selected = None;
    let mut channel = ModChannel::Alpha;
    for candidate in candidates {
        let Some((fit, candidate_channel)) = usable(candidate) else {
            continue;
        };
        if tagged && fit != LoaderFit::Tagged {
            continue;
        }
        if selected.is_none() || candidate_channel > channel {
            selected = Some(candidate);
            channel = candidate_channel;
        }
        if candidate_channel >= preferred {
            break;
        }
    }
    selected
}

pub(super) fn target_loaders(target: &DependencyTarget) -> Vec<&'static str> {
    accepted_loader_tags(&target.0, target.1.iter().map(|loader| loader.type_))
}

// ---------------------------------------------------------------------------
// What the instance already has.

/// Fabric API and Quilted Fabric API (Modrinth project, CurseForge project).
/// On Quilt either one provides the Fabric API (Prism's getOverrideDeps).
pub(crate) const FABRIC_API: (&str, i32) = ("P7dR8mSH", 306612);
pub(crate) const QUILTED_FABRIC_API: (&str, i32) = ("qvIfYCYJ", 634179);

/// Modrinth projects whose replaced pins are not reported. Every Fabric API
/// and Quilted Fabric API build is made for one Minecraft version and named
/// after it ("0.86.1+1.20.1"), so a mod listed for several releases pins the
/// build for one of them while it runs with any; the build for this game is
/// always another release, and warning about it every time teaches users to
/// ignore the warnings that matter. Only Modrinth dependencies carry pins.
pub(crate) const PIN_FREE_DEPENDENCIES: [&str; 2] = [FABRIC_API.0, QUILTED_FABRIC_API.0];

#[derive(Debug, Clone)]
pub(super) struct InstalledCopy {
    pub name: String,
    pub filename: String,
    pub mr_version_id: Option<String>,
}

impl InstalledCopy {
    fn of(row: &mfcdb::ModFullRow) -> Self {
        Self {
            name: row
                .cf_name
                .clone()
                .or_else(|| row.mr_title.clone())
                .or_else(|| row.meta_name.clone())
                .unwrap_or_else(|| row.filename.clone()),
            filename: row.filename.clone(),
            mr_version_id: row.mr_version_id.clone(),
        }
    }
}

#[derive(Debug)]
pub(super) enum InstalledDependency {
    /// At least one enabled copy: any version, duplicates and disabled
    /// siblings included.
    Enabled(Vec<InstalledCopy>),
    /// Only disabled copies, sorted by file name.
    DisabledOnly(Vec<InstalledCopy>),
    Absent,
}

/// Presence probe for a required project. It never bails on a version pin, on
/// stale hashes, on duplicate copies or on a copy whose file cannot be
/// inspected: those copies are all present, and the game loads them.
pub(super) async fn installed_dependency(
    app: &Arc<AppInner>,
    id: InstanceId,
    target: Option<&DependencyTarget>,
    cf: Option<i32>,
    mr: Option<&str>,
) -> anyhow::Result<InstalledDependency> {
    let quilt = target.is_some_and(|(_, loaders)| {
        loaders
            .iter()
            .any(|loader| loader.type_ == ModLoaderType::Quilt)
    });
    let fabric_api = quilt && (cf == Some(FABRIC_API.1) || mr == Some(FABRIC_API.0));
    probe(app, id, |row| {
        cf.is_some_and(|project| row.cf_project_id == Some(project))
            || mr.is_some_and(|project| row.mr_project_id.as_deref() == Some(project))
            || (fabric_api
                && (row.cf_project_id == Some(QUILTED_FABRIC_API.1)
                    || row.mr_project_id.as_deref() == Some(QUILTED_FABRIC_API.0)))
    })
    .await
}

/// A dependency declared by file name only is present when a copy with that
/// exact file name is.
async fn installed_file(
    app: &Arc<AppInner>,
    id: InstanceId,
    file_name: &str,
) -> anyhow::Result<InstalledDependency> {
    let found = probe(app, id, |row| row.filename == file_name).await?;
    if !matches!(found, InstalledDependency::Absent) {
        return Ok(found);
    }
    // A jar the scanner has not seen yet.
    let path = publication::current_path(app, id).await?;
    let folder = publication::checked_folder(&path, instance::AddonType::Mods).await?;
    let copy = || {
        vec![InstalledCopy {
            name: file_name.to_owned(),
            filename: file_name.to_owned(),
            mr_version_id: None,
        }]
    };
    Ok(match publication::selected_file(&folder, file_name).await {
        Ok(Some((_, true))) => InstalledDependency::Enabled(copy()),
        Ok(Some((_, false))) => InstalledDependency::DisabledOnly(copy()),
        Ok(None) => InstalledDependency::Absent,
        Err(error) => {
            tracing::debug!(
                file_name,
                "Required dependency file cannot be inspected: {error:#}"
            );
            InstalledDependency::Absent
        }
    })
}

/// An enabled jar whose mod id is the dependency's slug, although the scan
/// could not link it to the project: a CI build, a modpack override or a copy
/// from the other platform. Mod loaders refuse two jars with one mod id, so
/// downloading the project next to it would stop the game from starting.
async fn installed_mod_id(
    app: &Arc<AppInner>,
    id: InstanceId,
    slug: &str,
) -> anyhow::Result<Option<InstalledCopy>> {
    fn normalized(id: &str) -> String {
        id.trim().to_lowercase().replace('-', "_")
    }
    let slug = normalized(slug);
    if slug.is_empty() {
        return Ok(None);
    }
    let mods = instance::AddonType::Mods.to_db_string();
    let found = probe(app, id, |row| {
        row.addon_type == mods
            && row
                .modid
                .as_deref()
                .is_some_and(|modid| normalized(modid) == slug)
    })
    .await?;
    Ok(match found {
        InstalledDependency::Enabled(copies) => copies.into_iter().next(),
        _ => None,
    })
}

async fn probe(
    app: &Arc<AppInner>,
    id: InstanceId,
    claims: impl Fn(&mfcdb::ModFullRow) -> bool,
) -> anyhow::Result<InstalledDependency> {
    let rows = mfcdb::get_instance_mods_full(&app.db, *id).await?;
    let path = publication::current_path(app, id).await?;
    let mods = instance::AddonType::Mods.to_db_string();
    let mod_id = |row: &mfcdb::ModFullRow| {
        (row.addon_type == mods)
            .then(|| row.modid.as_deref().map(str::trim))
            .flatten()
            .filter(|modid| !modid.is_empty())
            .map(str::to_owned)
    };
    let mut enabled = Vec::new();
    let mut disabled = Vec::new();
    let mut disabled_mod_ids = HashSet::new();
    let mut unclaimed = Vec::new();
    for row in rows {
        if !claims(&row) {
            unclaimed.push(row);
            continue;
        }
        match current_state(&path, &row).await {
            // The file is gone; the row is stale.
            None => {}
            Some(true) => enabled.push(InstalledCopy::of(&row)),
            Some(false) => {
                disabled_mod_ids.extend(mod_id(&row));
                disabled.push(InstalledCopy::of(&row));
            }
        }
    }
    if enabled.is_empty() && !disabled_mod_ids.is_empty() {
        // The project's copy is disabled next to an enabled jar with the same
        // mod id that the scan does not link to the project (a build from
        // GitHub, or a copy linked only to the other platform). The Duplicate
        // mods resolver leaves exactly this behind. The game loads that jar,
        // and enabling the disabled copy as well, as the refusal would advise,
        // would stop it from starting with two mods of one id.
        for row in &unclaimed {
            if !mod_id(row).is_some_and(|modid| disabled_mod_ids.contains(&modid)) {
                continue;
            }
            if current_state(&path, row).await == Some(true) {
                tracing::info!(
                    filename = %row.filename,
                    modid = ?row.modid,
                    "An enabled jar with the mod id of a disabled required dependency provides it"
                );
                enabled.push(InstalledCopy::of(row));
            }
        }
    }
    disabled.sort_by(|a, b| a.filename.cmp(&b.filename));
    disabled.dedup_by(|a, b| a.filename == b.filename);
    Ok(if !enabled.is_empty() {
        InstalledDependency::Enabled(enabled)
    } else if !disabled.is_empty() {
        InstalledDependency::DisabledOnly(disabled)
    } else {
        InstalledDependency::Absent
    })
}

/// Whether a copy is enabled; `None` when its file is gone.
async fn current_state(path: &InstancePath, row: &mfcdb::ModFullRow) -> Option<bool> {
    match physical_state(path, row).await {
        Ok(state) => state,
        Err(error) => {
            // A symlinked jar, or an enabled and a disabled file of the same
            // name: the scanner saw the copy, so trust its recorded state.
            tracing::debug!(
                filename = %row.filename,
                "Required dependency copy cannot be inspected, using its recorded state: {error:#}"
            );
            Some(row.enabled)
        }
    }
}

async fn physical_state(
    path: &InstancePath,
    row: &mfcdb::ModFullRow,
) -> anyhow::Result<Option<bool>> {
    let kind = instance::AddonType::from_db_string(&row.addon_type)
        .context("Unknown required addon kind")?;
    let folder = publication::checked_folder(path, kind).await?;
    Ok(publication::selected_file(&folder, &row.filename)
        .await?
        .map(|(_, enabled)| enabled))
}

// ---------------------------------------------------------------------------
// What the version an update replaces already required.

/// Platform ids of the installed copy an update replaces.
#[derive(Debug, Clone, Default)]
pub(crate) struct ReplacedIds {
    /// CurseForge project and file.
    pub curseforge: Option<(i32, i32)>,
    pub modrinth_version: Option<String>,
}

/// The installed version an update replaces. Its required dependencies were
/// already the user's to manage: 2.0.40 never added any on update, and the
/// user may provide them with a jar the scan cannot link to the project, so
/// downloading the project again would install the same mod twice. Only the
/// requirements the new version introduces are resolved (GPT-0022).
pub(crate) struct Previous {
    version: PreviousVersion,
    declared: tokio::sync::OnceCell<HashSet<String>>,
}

enum PreviousVersion {
    Curseforge { project: i32, file: i32 },
    Modrinth { version: String },
}

impl Previous {
    pub(crate) fn curseforge(project: i32, file: i32) -> Arc<Self> {
        Arc::new(Self {
            version: PreviousVersion::Curseforge { project, file },
            declared: Default::default(),
        })
    }

    pub(crate) fn modrinth(version: String) -> Arc<Self> {
        Arc::new(Self {
            version: PreviousVersion::Modrinth { version },
            declared: Default::default(),
        })
    }

    /// Whether the replaced version required one of `keys` (planner keys, plus
    /// `modrinth-version:` / `modrinth-file:` for requirements without a
    /// project). Looked up once, and only when a dependency is missing.
    async fn declares(&self, catalogue: &dyn Catalogue, keys: &[String]) -> anyhow::Result<bool> {
        let declared = self
            .declared
            .get_or_try_init(|| async {
                let declared = match &self.version {
                    PreviousVersion::Curseforge { project, file } => {
                        catalogue.cf_file(*project, *file).await.map(|file| {
                            file.dependencies
                                .iter()
                                .filter(|dependency| {
                                    matches!(
                                        dependency.relation_type,
                                        curseforge::FileRelationType::RequiredDependency
                                    )
                                })
                                .map(|dependency| format!("curseforge:{}", dependency.mod_id))
                                .collect()
                        })
                    }
                    PreviousVersion::Modrinth { version } => {
                        catalogue.mr_version(version).await.map(|version| {
                            version
                                .dependencies
                                .iter()
                                .filter(|dependency| {
                                    dependency.dependency_type
                                        == modrinth::version::DependencyType::Required
                                })
                                .filter_map(|dependency| {
                                    match (
                                        &dependency.project_id,
                                        &dependency.version_id,
                                        &dependency.file_name,
                                    ) {
                                        (Some(project), _, _) => {
                                            Some(format!("modrinth:{project}"))
                                        }
                                        (None, Some(version), _) => {
                                            Some(format!("modrinth-version:{version}"))
                                        }
                                        (None, None, Some(file)) => {
                                            Some(format!("modrinth-file:{file}"))
                                        }
                                        (None, None, None) => None,
                                    }
                                })
                                .collect()
                        })
                    }
                };
                match declared {
                    Ok(declared) => Ok(declared),
                    // The old version is gone: resolve every missing requirement.
                    Err(error) if not_found(&error) => Ok(HashSet::new()),
                    Err(error) => Err(error.context(
                        "Could not look up the version this update replaces. Check your connection and retry.",
                    )),
                }
            })
            .await?;
        Ok(keys.iter().any(|key| declared.contains(key)))
    }
}

// ---------------------------------------------------------------------------
// Why a required dependency is not in the plan. The getters report it; the
// planner applies the policy (it knows whether this is an update).

#[derive(Debug, Clone, PartialEq)]
pub(super) struct SkippedDependency {
    /// Planner key (`platform:project`), to drop a skip whose project another
    /// part of the graph did resolve.
    pub key: String,
    /// Unknown when the platform no longer serves the project.
    pub name: Option<String>,
    pub platform: DependencyPlatform,
    pub project_id: String,
    pub reason: SkippedDependencyReason,
}

impl SkippedDependency {
    /// How logs and errors name it; the UI builds its own localized label.
    pub fn label(&self) -> String {
        label(self.name.as_deref(), self.platform, &self.project_id)
    }
}

/// How logs and errors name a dependency (instances and servers).
pub(crate) fn label(name: Option<&str>, platform: DependencyPlatform, project_id: &str) -> String {
    match name {
        Some(name) if name == project_id => name.to_owned(),
        Some(name) => format!("{name} ({project_id})"),
        None => match platform {
            DependencyPlatform::Curseforge => format!("CurseForge project {project_id}"),
            DependencyPlatform::Modrinth => format!("Modrinth project {project_id}"),
        },
    }
}

impl From<SkippedDependency> for TaskWarning {
    fn from(value: SkippedDependency) -> Self {
        TaskWarning::SkippedDependency {
            name: value.name,
            platform: value.platform,
            project_id: value.project_id,
            reason: value.reason,
        }
    }
}

/// A Modrinth pin that is not for this game and loader, replaced by another
/// release of the dependency. The addon may refuse that release when the game
/// starts (Iris 1.11.4 needs Sodium 0.9 but 26.1.1 only has 0.8.9), so the
/// user is told which version it asked for.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReplacedPin {
    /// Planner key of the dependency (`modrinth:project`).
    pub key: String,
    /// Planner identity of the version installed instead of the pin.
    pub identity: String,
    pub name: String,
    pub project_id: String,
    /// Version numbers as Modrinth shows them.
    pub pinned: String,
    pub installed: String,
}

impl From<ReplacedPin> for TaskWarning {
    fn from(value: ReplacedPin) -> Self {
        TaskWarning::ReplacedPinnedVersion {
            name: value.name,
            project_id: value.project_id,
            pinned: value.pinned,
            installed: value.installed,
        }
    }
}

/// A dependency that two dependants in one installation pin at different
/// versions. Only one copy can be installed, so the planner installs the
/// newest pinned version and tells the user which one, as server installs do.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConflictingPin {
    pub name: String,
    pub platform: DependencyPlatform,
    pub project_id: String,
    /// The version installed, as the platform shows it.
    pub installed: String,
}

impl From<ConflictingPin> for TaskWarning {
    fn from(value: ConflictingPin) -> Self {
        TaskWarning::ConflictingVersions {
            name: value.name,
            platform: value.platform,
            project_id: value.project_id,
            installed: value.installed,
        }
    }
}

/// A version number without the Minecraft versions and loaders it names, so
/// "mc1.20.1-0.5.11" and "mc1.20.6-0.5.11" are one release of Sodium built
/// for two Minecraft versions.
pub(crate) fn release_of(version: &modrinth::version::Version) -> Vec<String> {
    let number = version.version_number.to_lowercase();
    number
        .split(['-', '+', '_', ' '])
        .filter(|token| !token.is_empty())
        .filter(|token| {
            !token
                .strip_prefix("mc")
                .is_some_and(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
        })
        .filter(|token| {
            !version
                .game_versions
                .iter()
                .any(|game| game.eq_ignore_ascii_case(token))
        })
        .filter(|token| !LOADER_TAGS.contains(token))
        .map(|token| {
            token
                .strip_prefix('v')
                .filter(|rest| rest.starts_with(|c: char| c.is_ascii_digit()))
                .unwrap_or(token)
                .to_owned()
        })
        .collect()
}

/// Mod loaders a multi-loader upload can name.
const MOD_LOADERS: &[&str] = &["forge", "neoforge", "fabric", "quilt"];

/// One multi-loader upload declares the requirements of every loader's build:
/// a LambDynamicLights jar for Fabric, Quilt and NeoForge requires Fabric API,
/// which only its Fabric build loads. A dependency that only exists for the
/// parent's other loaders is not needed by the instance (and installing it
/// would add a mod the instance cannot load), so it is left out silently.
fn for_another_loader(parent: &[String], dependency: &[String], accepted: &[&str]) -> bool {
    let loaders = |tags: &[String]| -> HashSet<String> {
        tags.iter()
            .map(|tag| tag.to_lowercase())
            .filter(|tag| MOD_LOADERS.contains(&tag.as_str()))
            .collect()
    };
    let parent = loaders(parent);
    let dependency = loaders(dependency);
    let runs_here = parent.iter().any(|tag| accepted.contains(&tag.as_str()));
    let only_elsewhere = !dependency.is_empty()
        && dependency
            .iter()
            .all(|tag| !accepted.contains(&tag.as_str()));
    runs_here
        && only_elsewhere
        && dependency
            .iter()
            .any(|tag| parent.contains(tag) && !accepted.contains(&tag.as_str()))
}

#[derive(Debug, Clone, thiserror::Error)]
pub(super) enum DependencyGap {
    /// Only disabled copies are installed. The planner installs the addon
    /// next to them, and a fresh install names `dependency` in a warning.
    #[error("{}", disabled_message(.dependency.name.as_deref().unwrap_or(&.dependency.project_id), .filenames))]
    DisabledOnly {
        /// Its reason is [`SkippedDependencyReason::Disabled`].
        dependency: SkippedDependency,
        filenames: Vec<String>,
    },
    #[error("Required dependency {} cannot be installed: {}", .0.label(), describe(.0.reason))]
    Unresolvable(SkippedDependency),
}

fn disabled_message(name: &str, filenames: &[String]) -> String {
    match filenames {
        [filename] => format!(
            "Required dependency {name} ({filename}) is installed but disabled. Enable it in Addons and retry."
        ),
        _ => format!(
            "Required dependency {name} is installed but disabled ({}). Enable one of them in Addons and retry.",
            filenames.join(", ")
        ),
    }
}

/// Why a dependency is left out, for logs and errors (instances and servers).
pub(crate) fn describe(reason: SkippedDependencyReason) -> &'static str {
    match reason {
        SkippedDependencyReason::NoCompatibleVersion => {
            "it has no version for this Minecraft version and loader"
        }
        SkippedDependencyReason::NotDownloadable => {
            "its author only allows downloads from the CurseForge website"
        }
        SkippedDependencyReason::Unavailable => "it is not available for download",
        SkippedDependencyReason::NotForServers => "it does not run on servers",
        SkippedDependencyReason::RemovedDuringInstall => {
            "it was removed while the installation was running"
        }
        SkippedDependencyReason::Disabled => "it is installed but disabled",
    }
}

impl DependencyGap {
    /// `skip` names the requirement; the copies' own name is how the user
    /// sees it in Addons.
    fn disabled(skip: &Skip, copies: Vec<InstalledCopy>) -> anyhow::Error {
        Self::DisabledOnly {
            dependency: SkippedDependency {
                key: skip.key.clone(),
                name: copies
                    .first()
                    .map(|copy| copy.name.clone())
                    .or_else(|| skip.name.clone()),
                platform: skip.platform,
                project_id: skip.project_id.clone(),
                reason: SkippedDependencyReason::Disabled,
            },
            filenames: copies.into_iter().map(|copy| copy.filename).collect(),
        }
        .into()
    }

    /// A dependency the plan would download whose file is already in the
    /// folder under the same name, disabled, although the scan has not linked
    /// it to the project yet.
    pub(super) fn disabled_file(resource: &dyn ResourceInstaller, filename: &str) -> anyhow::Error {
        let identity = resource.id();
        let mut fields = identity.split(':');
        let (platform, project_id) = match (fields.next(), fields.next()) {
            (Some("curseforge"), Some(project)) => (DependencyPlatform::Curseforge, project),
            (Some(_), Some(project)) => (DependencyPlatform::Modrinth, project),
            _ => (DependencyPlatform::Modrinth, filename),
        };
        let key = identity
            .rsplit_once(':')
            .map_or(identity.as_str(), |(key, _)| key)
            .to_owned();
        Self::DisabledOnly {
            dependency: SkippedDependency {
                key,
                name: Some(resource.project_name()),
                platform,
                project_id: project_id.to_owned(),
                reason: SkippedDependencyReason::Disabled,
            },
            filenames: vec![filename.to_owned()],
        }
        .into()
    }
}

/// Builds the skip for one dependency while its name is being discovered.
struct Skip {
    key: String,
    name: Option<String>,
    platform: DependencyPlatform,
    project_id: String,
}

impl Skip {
    fn curseforge(project_id: i32) -> Self {
        Self {
            key: format!("curseforge:{project_id}"),
            name: None,
            platform: DependencyPlatform::Curseforge,
            project_id: project_id.to_string(),
        }
    }

    fn modrinth(project_id: &str) -> Self {
        Self {
            key: format!("modrinth:{project_id}"),
            name: None,
            platform: DependencyPlatform::Modrinth,
            project_id: project_id.to_owned(),
        }
    }

    fn because(&self, reason: SkippedDependencyReason) -> anyhow::Error {
        DependencyGap::Unresolvable(SkippedDependency {
            key: self.key.clone(),
            name: self.name.clone(),
            platform: self.platform,
            project_id: self.project_id.clone(),
            reason,
        })
        .into()
    }

    /// A project, file or version the platform no longer serves is skipped.
    /// Any other lookup failure (offline, timeout, server error) stays fatal so
    /// a transient problem never installs an addon without its dependency.
    fn lookup(&self, error: anyhow::Error) -> anyhow::Error {
        if not_found(&error) {
            self.because(SkippedDependencyReason::Unavailable)
        } else {
            error.context(format!(
                "Could not look up required dependency {}. Check your connection and retry.",
                label(self.name.as_deref(), self.platform, &self.project_id)
            ))
        }
    }
}

/// Whether a platform lookup failed because the project, file or version is
/// gone (404/410), as opposed to a connection or server problem.
pub(crate) fn not_found(error: &anyhow::Error) -> bool {
    use crate::error::request::{RequestError, RequestErrorDetails};
    error.chain().any(|cause| {
        cause.downcast_ref::<RequestError>().is_some_and(|request| {
            matches!(
                request.error,
                RequestErrorDetails::UnexpectedStatus { status, .. }
                    if status == reqwest::StatusCode::NOT_FOUND || status == reqwest::StatusCode::GONE
            )
        })
    })
}

// ---------------------------------------------------------------------------
// Platform lookups, behind a seam so the resolution can be tested offline.

#[async_trait::async_trait]
pub(crate) trait Catalogue: Send + Sync {
    async fn cf_mod(&self, project: i32) -> anyhow::Result<curseforge::Mod>;
    async fn cf_file(&self, project: i32, file: i32) -> anyhow::Result<curseforge::File>;
    async fn cf_files(
        &self,
        project: i32,
        release: &str,
        index: i32,
    ) -> anyhow::Result<CurseForgeResponse<Vec<curseforge::File>>>;
    async fn mr_version(&self, version: &str) -> anyhow::Result<modrinth::version::Version>;
    async fn mr_project(&self, project: &str) -> anyhow::Result<modrinth::project::Project>;
    async fn mr_versions(
        &self,
        filters: ProjectVersionsFilters,
    ) -> anyhow::Result<Vec<modrinth::version::Version>>;
}

/// CurseForge and Modrinth through the app's platform clients.
pub(crate) struct Live(pub Arc<AppInner>);

#[async_trait::async_trait]
impl Catalogue for Live {
    async fn cf_mod(&self, project: i32) -> anyhow::Result<curseforge::Mod> {
        Ok(self
            .0
            .modplatforms_manager()
            .curseforge
            .get_mod(ModParameters { mod_id: project })
            .await?
            .data)
    }
    async fn cf_file(&self, project: i32, file: i32) -> anyhow::Result<curseforge::File> {
        Ok(self
            .0
            .modplatforms_manager()
            .curseforge
            .get_mod_file(ModFileParameters {
                mod_id: project,
                file_id: file,
            })
            .await?
            .data)
    }
    async fn cf_files(
        &self,
        project: i32,
        release: &str,
        index: i32,
    ) -> anyhow::Result<CurseForgeResponse<Vec<curseforge::File>>> {
        self.0
            .modplatforms_manager()
            .curseforge
            .get_mod_files(ModFilesParameters {
                mod_id: project,
                query: ModFilesParametersQuery {
                    game_version: Some(release.to_owned()),
                    game_version_type_id: None,
                    index: Some(index),
                    page_size: Some(50),
                    mod_loader_type: None,
                },
            })
            .await
    }
    async fn mr_version(&self, version: &str) -> anyhow::Result<modrinth::version::Version> {
        self.0
            .modplatforms_manager()
            .modrinth
            .get_version(VersionID(version.to_owned()))
            .await
    }
    async fn mr_project(&self, project: &str) -> anyhow::Result<modrinth::project::Project> {
        self.0
            .modplatforms_manager()
            .modrinth
            .get_project(ProjectID(project.to_owned()))
            .await
    }
    async fn mr_versions(
        &self,
        filters: ProjectVersionsFilters,
    ) -> anyhow::Result<Vec<modrinth::version::Version>> {
        Ok(self
            .0
            .modplatforms_manager()
            .modrinth
            .get_project_versions(filters)
            .await?
            .0)
    }
}

// ---------------------------------------------------------------------------
// Resolution of one required dependency.

#[derive(Clone)]
pub(super) struct Request {
    pub app: Arc<AppInner>,
    pub instance_id: InstanceId,
    pub target: Option<DependencyTarget>,
    pub preferred: ModChannel,
    pub catalogue: Arc<dyn Catalogue>,
    /// Set on the root of an update: the version it replaces.
    pub previous: Option<Arc<Previous>>,
    /// Loader tags of the version that declares the requirements (Modrinth
    /// `loaders`, CurseForge `gameVersions`).
    pub parent_loaders: Vec<String>,
}

impl Request {
    /// A requirement of the parent's build for another loader (see
    /// [`for_another_loader`]).
    fn for_another_loader(&self, name: &str, dependency_loaders: &[String]) -> bool {
        let Some(target) = self.target.as_ref() else {
            return false;
        };
        let other = for_another_loader(
            &self.parent_loaders,
            dependency_loaders,
            &target_loaders(target),
        );
        if other {
            tracing::info!(
                dependency = name,
                parent_loaders = ?self.parent_loaders,
                "Leaving out a required dependency that only the parent's build for another loader uses"
            );
        }
        other
    }

    async fn installed(
        &self,
        cf: Option<i32>,
        mr: Option<&str>,
    ) -> anyhow::Result<InstalledDependency> {
        installed_dependency(&self.app, self.instance_id, self.target.as_ref(), cf, mr).await
    }

    /// Whether the version an update replaces already required this
    /// dependency, which the user then manages (see [`Previous`]).
    async fn declared_before(&self, keys: &[String]) -> anyhow::Result<bool> {
        let Some(previous) = &self.previous else {
            return Ok(false);
        };
        let declared = previous.declares(&*self.catalogue, keys).await?;
        if declared {
            tracing::info!(
                dependency = %keys[0],
                "Updating without a required dependency the replaced version already required"
            );
        }
        Ok(declared)
    }

    /// A jar with the project's slug as its mod id (see [`installed_mod_id`]).
    async fn provided_by_mod_id(&self, slug: &str) -> anyhow::Result<bool> {
        let Some(copy) = installed_mod_id(&self.app, self.instance_id, slug).await? else {
            return Ok(false);
        };
        tracing::info!(
            dependency = slug,
            filename = %copy.filename,
            "Keeping an installed jar with the required dependency's mod id"
        );
        Ok(true)
    }
}

/// Pages of 50 CurseForge files (100 Modrinth versions) a dependency lookup reads at most.
pub(crate) const PAGES: i32 = 20;

/// `Ok(None)`: already satisfied by the instance, nothing to plan.
pub(super) async fn curseforge(
    request: &Request,
    project_id: i32,
    selections: &HashMap<String, String>,
) -> anyhow::Result<Option<BoxedResourceInstaller>> {
    let mut skip = Skip::curseforge(project_id);
    let planned = selections
        .get(&skip.key)
        .and_then(|id| id.rsplit(':').next())
        .map(str::parse::<i32>)
        .transpose()?;
    if planned.is_none() {
        match request.installed(Some(project_id), None).await? {
            InstalledDependency::Enabled(_) => return Ok(None),
            InstalledDependency::DisabledOnly(copies) => {
                return Err(DependencyGap::disabled(&skip, copies));
            }
            InstalledDependency::Absent => {}
        }
        if request.declared_before(&[skip.key.clone()]).await? {
            return Ok(None);
        }
    }
    let catalogue = &request.catalogue;
    let project = catalogue
        .cf_mod(project_id)
        .await
        .map_err(|error| skip.lookup(error))?;
    skip.name = Some(project.name.clone());
    if project.id != project_id {
        return Err(skip.because(SkippedDependencyReason::Unavailable));
    }
    if planned.is_none() && request.provided_by_mod_id(&project.slug).await? {
        return Ok(None);
    }
    let mut files = Vec::new();
    if let Some(file_id) = planned {
        files.push(
            catalogue
                .cf_file(project_id, file_id)
                .await
                .map_err(|error| skip.lookup(error))?,
        );
    } else {
        let Some((release, _)) = request.target.as_ref() else {
            return Err(skip.because(SkippedDependencyReason::NoCompatibleVersion));
        };
        for page in 0..PAGES {
            let response = catalogue
                .cf_files(project_id, release, page * 50)
                .await
                .map_err(|error| skip.lookup(error))?;
            let done = response.data.len() < 50
                || response
                    .pagination
                    .as_ref()
                    .is_some_and(|p| p.index + p.result_count >= p.total_count);
            files.extend(response.data);
            if done {
                break;
            }
            if page == PAGES - 1 {
                return Err(skip.because(SkippedDependencyReason::Unavailable));
            }
        }
    }
    if files.iter().any(|file| file.mod_id != project_id) {
        return Err(skip.because(SkippedDependencyReason::Unavailable));
    }
    let independent = matches!(
        project.class_id,
        Some(
            curseforge::ClassId::ResourcePacks
                | curseforge::ClassId::Shaders
                | curseforge::ClassId::Datapacks
                | curseforge::ClassId::Worlds
        )
    );
    // Never fall back to an incompatible file (GPT-0071).
    let Ok(file) = select_dependency(
        &files,
        request.target.as_ref(),
        independent,
        request.preferred,
        |file| {
            (
                &file.game_versions,
                &file.game_versions,
                ModChannel::from(file.release_type),
            )
        },
    ) else {
        let tags: Vec<String> = files
            .iter()
            .flat_map(|file| file.game_versions.iter().cloned())
            .collect();
        if !independent && request.for_another_loader(&project.name, &tags) {
            return Ok(None);
        }
        return Err(skip.because(SkippedDependencyReason::NoCompatibleVersion));
    };
    let Ok(mut resource) = CurseforgeModInstaller::from_file(file.clone(), project) else {
        return Err(skip.because(SkippedDependencyReason::NotDownloadable));
    };
    resource.exact_dependency = false;
    #[cfg(test)]
    {
        resource.catalogue = Some(request.catalogue.clone());
    }
    if planned.is_none() {
        validate_required_resource(&resource, &request.app, request.instance_id).await?;
    }
    Ok(Some(Box::new(resource)))
}

fn mr_metadata(version: &modrinth::version::Version) -> (&Vec<String>, &Vec<String>, ModChannel) {
    (
        &version.game_versions,
        &version.loaders,
        ModChannel::from(version.version_type),
    )
}

/// `Ok(None)`: already satisfied by the instance, nothing to plan.
pub(super) async fn modrinth(
    request: &Request,
    dependency: &modrinth::version::Dependency,
    selections: &HashMap<String, String>,
) -> anyhow::Result<Option<BoxedResourceInstaller>> {
    let catalogue = &request.catalogue;
    let declared = dependency.version_id.clone();
    let (mut skip, mut fetched) = match (&dependency.project_id, &declared) {
        (Some(project), _) => (Skip::modrinth(project), None),
        (None, Some(pin)) => {
            // Declared by version only: the pin names the project.
            let mut skip = Skip::modrinth(pin);
            skip.key = format!("modrinth-version:{pin}");
            let version = catalogue
                .mr_version(pin)
                .await
                .map_err(|error| skip.lookup(error))?;
            if version.id != *pin {
                return Err(skip.because(SkippedDependencyReason::Unavailable));
            }
            (Skip::modrinth(&version.project_id), Some(version))
        }
        (None, None) => return by_file_name(request, dependency.file_name.as_deref()).await,
    };
    let project_id = skip.project_id.clone();
    let planned = selections
        .get(&skip.key)
        .and_then(|id| id.rsplit(':').next())
        .map(str::to_owned);
    // Check the instance before any request whenever the project is known.
    if planned.is_none() {
        if satisfied(request, &skip, declared.as_deref()).await? {
            return Ok(None);
        }
        let mut keys = vec![skip.key.clone()];
        keys.extend(declared.iter().map(|pin| format!("modrinth-version:{pin}")));
        if request.declared_before(&keys).await? {
            return Ok(None);
        }
    }
    let project = catalogue
        .mr_project(&project_id)
        .await
        .map_err(|error| skip.lookup(error))?;
    skip.name = Some(project.title.clone());
    if project.id != project_id {
        return Err(skip.because(SkippedDependencyReason::Unavailable));
    }
    if planned.is_none() && request.provided_by_mod_id(&project.slug).await? {
        return Ok(None);
    }
    let is_mod = project.project_type == modrinth::project::ProjectType::Mod;
    let fits = |version: &modrinth::version::Version| {
        select_dependency(
            std::slice::from_ref(version),
            request.target.as_ref(),
            !is_mod,
            request.preferred,
            mr_metadata,
        )
        .is_ok()
    };
    // The declared pin, then the version another dependant settled on. A pin
    // that is gone or is not for this game and loader is replaced by the
    // newest compatible version, never by an incompatible file (GPT-0071).
    let mut candidates: Vec<String> = Vec::new();
    for pin in [declared.clone(), planned.clone()].into_iter().flatten() {
        if !candidates.contains(&pin) {
            candidates.push(pin);
        }
    }
    let mut chosen = None;
    // The declared pin, when it is not for this game and loader.
    let mut unfit_pin = None;
    for pin in candidates {
        let version = match fetched.take() {
            Some(version) if version.id == pin => version,
            _ => match catalogue.mr_version(&pin).await {
                Ok(version) => version,
                Err(error) if not_found(&error) => {
                    tracing::info!(
                        dependency = %project.title,
                        pin = %pin,
                        "A pinned required version is no longer available; choosing a compatible one"
                    );
                    continue;
                }
                Err(error) => return Err(skip.lookup(error)),
            },
        };
        if version.project_id != project_id || version.id != pin {
            tracing::warn!(
                dependency = %project.title,
                pin = %pin,
                "A pinned required version belongs to another project; choosing a compatible one"
            );
            continue;
        }
        if fits(&version) {
            chosen = Some((version, declared.as_ref() == Some(&pin)));
            break;
        }
        tracing::info!(
            dependency = %project.title,
            pin = %pin,
            version = %version.version_number,
            "A pinned required version is not for this Minecraft version and loader; choosing a compatible one"
        );
        if declared.as_ref() == Some(&pin) {
            unfit_pin = Some(version);
        }
    }
    let (version, exact) = match chosen {
        Some(chosen) => chosen,
        None => {
            let Some(target) = request.target.as_ref() else {
                return Err(skip.because(SkippedDependencyReason::NoCompatibleVersion));
            };
            let loaders = target_loaders(target);
            if is_mod && loaders.is_empty() {
                // A vanilla instance cannot load any mod; don't ask for one.
                return Err(skip.because(SkippedDependencyReason::NoCompatibleVersion));
            }
            let mut versions = Vec::new();
            for page in 0..PAGES {
                let response = catalogue
                    .mr_versions(ProjectVersionsFilters {
                        project_id: ProjectID(project_id.clone()),
                        game_versions: Some(vec![target.0.clone()]),
                        loaders: is_mod.then(|| loaders.iter().map(|l| (*l).to_owned()).collect()),
                        limit: Some(100),
                        offset: Some(page as u32 * 100),
                    })
                    .await
                    .map_err(|error| skip.lookup(error))?;
                let done = response.len() < 100;
                versions.extend(response);
                if done {
                    break;
                }
                if page == PAGES - 1 {
                    return Err(skip.because(SkippedDependencyReason::Unavailable));
                }
            }
            if versions.iter().any(|v| v.project_id != project_id) {
                return Err(skip.because(SkippedDependencyReason::Unavailable));
            }
            // The pinned release built for this Minecraft version, if there is
            // one, before the newest compatible version.
            let release = unfit_pin.as_ref().map(release_of).unwrap_or_default();
            let same_release: Vec<_> = if release.is_empty() {
                Vec::new()
            } else {
                versions
                    .iter()
                    .filter(|version| release_of(version) == release)
                    .cloned()
                    .collect()
            };
            let select = |versions: &[modrinth::version::Version]| {
                select_dependency(
                    versions,
                    request.target.as_ref(),
                    !is_mod,
                    request.preferred,
                    mr_metadata,
                )
                .ok()
                .cloned()
            };
            let Some(version) = select(&same_release).or_else(|| select(&versions)) else {
                if is_mod && request.for_another_loader(&project.title, &project.loaders) {
                    return Ok(None);
                }
                return Err(skip.because(SkippedDependencyReason::NoCompatibleVersion));
            };
            (version, false)
        }
    };
    // A version number that only names a Minecraft version says nothing
    // about the release, so it is reported.
    let replaced = unfit_pin
        .filter(|_| !PIN_FREE_DEPENDENCIES.contains(&project_id.as_str()))
        .filter(|pin| {
            let pinned = release_of(pin);
            pinned.is_empty() || pinned != release_of(&version)
        })
        .map(|pin| ReplacedPin {
            key: skip.key.clone(),
            identity: format!("modrinth:{project_id}:{}", version.id),
            name: project.title.clone(),
            project_id: project_id.clone(),
            pinned: pin.version_number,
            installed: version.version_number.clone(),
        });
    let Ok(mut resource) = ModrinthModInstaller::from_version(version, project) else {
        return Err(skip.because(SkippedDependencyReason::Unavailable));
    };
    resource.exact_dependency = exact;
    resource.replaced_pin = replaced;
    #[cfg(test)]
    {
        resource.catalogue = Some(request.catalogue.clone());
    }
    if planned.is_none() {
        validate_required_resource(&resource, &request.app, request.instance_id).await?;
    }
    Ok(Some(Box::new(resource)))
}

/// A Modrinth dependency declared by file name only: nothing on Modrinth can
/// provide it, so it is satisfied by a copy with that name or reported.
async fn by_file_name(
    request: &Request,
    file_name: Option<&str>,
) -> anyhow::Result<Option<BoxedResourceInstaller>> {
    let Some(file_name) = file_name.filter(|name| !name.trim().is_empty()) else {
        tracing::debug!("Ignoring a required dependency that names no project, version or file");
        return Ok(None);
    };
    let skip = Skip {
        key: format!("modrinth-file:{file_name}"),
        name: Some(file_name.to_owned()),
        platform: DependencyPlatform::Modrinth,
        project_id: file_name.to_owned(),
    };
    match installed_file(&request.app, request.instance_id, file_name).await? {
        InstalledDependency::Enabled(_) => return Ok(None),
        InstalledDependency::DisabledOnly(copies) => {
            return Err(DependencyGap::disabled(&skip, copies));
        }
        InstalledDependency::Absent => {}
    }
    if request.declared_before(&[skip.key.clone()]).await? {
        return Ok(None);
    }
    Err(skip.because(SkippedDependencyReason::Unavailable))
}

/// Whether the instance already provides a Modrinth project. A pin is only
/// used to choose what to download; an installed copy of another version is
/// kept, as Prism and the Modrinth App do.
async fn satisfied(request: &Request, skip: &Skip, pin: Option<&str>) -> anyhow::Result<bool> {
    match request.installed(None, Some(&skip.project_id)).await? {
        InstalledDependency::Enabled(copies) => {
            if let Some(pin) = pin {
                if !copies
                    .iter()
                    .any(|copy| copy.mr_version_id.as_deref() == Some(pin))
                {
                    tracing::info!(
                        dependency = %copies[0].name,
                        filename = %copies[0].filename,
                        pin,
                        "Keeping the installed required dependency although another version is pinned"
                    );
                }
            }
            Ok(true)
        }
        InstalledDependency::DisabledOnly(copies) => Err(DependencyGap::disabled(skip, copies)),
        InstalledDependency::Absent => Ok(false),
    }
}
