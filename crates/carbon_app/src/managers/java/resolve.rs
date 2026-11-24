//! The automatic Java choice: the Java an instance on Automatic launches with.
//! The launch path, the Java overview and the instance's resolved Java all go
//! through [`resolve_automatic`], so they never disagree.

use super::{
    advisories::{RealHostCpu, match_advisories},
    build_version::JavaBuildVersion,
    invalidate_java_lists,
    major_defaults::get_major_defaults,
    scan_and_sync::sync_system_java_profiles,
};
use crate::{
    App,
    domain::{
        instance::info::{JavaOverride, ModLoaderType, StandardVersion},
        java::{
            Java, JavaArch, JavaComponent, JavaComponentType, JavaProfile, JavaVendor,
            SystemJavaProfileName,
        },
    },
};
use carbon_repos::{
    db_exec::Db,
    repos::java::{self as java_repo, JavaRow},
};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};
use strum::IntoEnumIterator;
use tokio::sync::OnceCell;

/// The Java builds a system profile accepts, or an instance with a Java floor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaRequirement {
    pub major: u8,
    /// When set, builds of `major` must be strictly below it.
    pub below: Option<JavaBuildVersion>,
    /// Every later major is accepted too (see [`accepted_requirements`]).
    pub or_later: bool,
}

impl JavaRequirement {
    /// Any build of `major`, and only of it.
    pub fn of_major(major: u8) -> Self {
        Self {
            major,
            below: None,
            or_later: false,
        }
    }

    pub fn accepts(&self, major: u8, version: &JavaBuildVersion) -> bool {
        match major.cmp(&self.major) {
            std::cmp::Ordering::Equal => self.below.as_ref().is_none_or(|below| version < below),
            std::cmp::Ordering::Greater => self.or_later,
            std::cmp::Ordering::Less => false,
        }
    }

    /// Whether some build of `major` is accepted.
    pub fn accepts_major(&self, major: u8) -> bool {
        major == self.major || (self.or_later && major > self.major)
    }

    pub fn accepts_component(&self, component: &JavaComponent) -> bool {
        let major = u8::try_from(component.version.major).ok();
        let version = JavaBuildVersion::parse(&component.version.to_string());

        matches!((major, version), (Some(major), Some(version)) if self.accepts(major, &version))
    }
}

pub fn requirement_for(profile: SystemJavaProfileName) -> JavaRequirement {
    let below = match profile {
        // Forge 1.16.5 breaks on Java 8 update 312 and later.
        SystemJavaProfileName::LegacyFixed1 => {
            Some(JavaBuildVersion::from_components(&[8, 0, 312]))
        }
        _ => None,
    };

    JavaRequirement {
        major: profile.major(),
        below,
        or_later: false,
    }
}

/// The profile a launch of `version` really needs, given the `required` one its
/// version asks for: Forge on 1.16.5 breaks on Java 8 update 312 and later.
pub fn effective_profile(
    required: SystemJavaProfileName,
    version: &StandardVersion,
) -> SystemJavaProfileName {
    if needs_legacy_fixed_1(version) {
        SystemJavaProfileName::LegacyFixed1
    } else {
        required
    }
}

fn needs_legacy_fixed_1(version: &StandardVersion) -> bool {
    version.release == "1.16.5"
        && version
            .modloaders
            .iter()
            .any(|loader| loader.type_ == ModLoaderType::Forge)
}

/// The requirement that caps the build within a major for an instance of
/// `version`, if it has one. Only [`effective_profile`]'s patch does: Forge
/// 1.16.5 needs Java 8 below update 312. So moving the instance to another
/// build of the same major can only break it here, and that needs no version
/// data.
pub fn bounded_requirement(version: &StandardVersion) -> Option<JavaRequirement> {
    needs_legacy_fixed_1(version).then(|| requirement_for(SystemJavaProfileName::LegacyFixed1))
}

/// What a launch does with an instance's Java override.
#[derive(Debug, Clone, PartialEq)]
pub enum OverrideChoice {
    /// The binary at this path, set in the instance.
    Path(String),
    /// The Java a profile the instance names links to.
    Linked {
        id: String,
        component: JavaComponent,
    },
    /// The automatic choice for the instance itself: there is no override, it
    /// names the required system profile, its profile no longer exists, or
    /// it's a custom profile whose Java is gone or doesn't start.
    Automatic,
    /// The automatic choice for another system profile the instance names,
    /// which has no usable Java linked.
    ProfileAutomatic(SystemJavaProfileName),
}

/// Whether `path`, a `Path` override, names a GDLauncher-installed Java
/// that is gone: inside `managed_root`, and missing. Updates and deletes
/// repoint the instances that use a Java, but not one that wasn't loaded
/// then or that spells the path differently.
pub fn is_missing_managed_java(path: &str, managed_root: &Path) -> bool {
    let path = Path::new(path);
    path.starts_with(managed_root) && !path.exists()
}

/// What `java_override` resolves to. `required` is the system profile the
/// instance's Minecraft version asks for, before [`effective_profile`], when
/// known: the frontend stores it as a `Profile` override to mean Automatic.
/// `managed_root` is where GDLauncher installs Javas.
pub fn override_choice(
    java_override: Option<&JavaOverride>,
    required: Option<SystemJavaProfileName>,
    profiles: &[JavaProfile],
    javas: &[Java],
    managed_root: &Path,
) -> OverrideChoice {
    let name = match java_override {
        // The old instance settings stored these for Path or Profile mode
        // before anything was chosen. Instance settings show them as
        // Automatic, so a launch uses Automatic too.
        None | Some(JavaOverride::Path(None) | JavaOverride::Profile(None)) => {
            return OverrideChoice::Automatic;
        }
        // A GDLauncher Java that is gone means Automatic, never a failed
        // launch. A path of the user's, such as on a drive that isn't
        // mounted, is theirs to fix.
        Some(JavaOverride::Path(Some(path))) if is_missing_managed_java(path, managed_root) => {
            return OverrideChoice::Automatic;
        }
        Some(JavaOverride::Path(Some(path))) => return OverrideChoice::Path(path.clone()),
        Some(JavaOverride::Profile(Some(name))) => name,
    };

    if required.is_some_and(|required| *name == required.to_string()) {
        return OverrideChoice::Automatic;
    }

    let Some(profile) = profiles.iter().find(|profile| profile.name == *name) else {
        return OverrideChoice::Automatic;
    };
    let linked = profile
        .java_id
        .as_ref()
        .and_then(|java_id| javas.iter().find(|java| &java.id == java_id));

    // Sync only links GDLauncher's own Javas and defaults, so a system profile
    // may well be empty: it stands for that Java version's automatic choice.
    if profile.is_system
        && !linked.is_some_and(|java| java.is_valid)
        && let Ok(system) = SystemJavaProfileName::try_from(name.as_str())
    {
        return OverrideChoice::ProfileAutomatic(system);
    }
    // Deleting a Java empties the custom profiles linked to it. Like a deleted
    // profile, one with nothing that starts means Automatic, never a failed
    // launch.
    if !profile.is_system && !linked.is_some_and(|java| java.is_valid) {
        return OverrideChoice::Automatic;
    }

    match linked {
        Some(java) => OverrideChoice::Linked {
            id: java.id.clone(),
            component: java.component.clone(),
        },
        // A system profile under a name no Minecraft version uses.
        None => OverrideChoice::Automatic,
    }
}

/// The requirement a Java of `major` serves for `profile`: the profile's own,
/// or plain `major` when [`effective_major`] swapped the major or a Java
/// floor raised it.
pub fn requirement_for_major(profile: SystemJavaProfileName, major: u8) -> JavaRequirement {
    let req = requirement_for(profile);
    if req.major == major {
        req
    } else {
        JavaRequirement::of_major(major)
    }
}

/// The major an instance's Java floor raises its need to: the floor a
/// wrong-Java crash showed (see `GameConfig::java_floor`), never below what
/// `profile` needs.
pub fn floor_major(profile: SystemJavaProfileName, floor: Option<u8>) -> Option<u8> {
    floor.map(|floor| floor.max(profile.major()))
}

/// The requirement Automatic serves an instance that needs `profile` with
/// first: its floor's major when it has one (see [`floor_major`]), else the
/// profile's own.
pub fn need_requirement(profile: SystemJavaProfileName, floor: Option<u8>) -> JavaRequirement {
    match floor_major(profile, floor) {
        Some(major) => requirement_for_major(profile, major),
        None => requirement_for(profile),
    }
}

/// The Javas an instance that needs `profile` runs on. This decides what
/// fits an instance, for the mismatch check, the dropdown and the launch's
/// warning alike:
/// - with a Java `floor`, every major from [`floor_major`] up, the cap on
///   its builds kept (Forge 1.16.5);
/// - otherwise the profile's own requirement, and the one
///   [`effective_major`] swaps in where a catalog lacks the major. So
///   Minecraft 1.17 takes Java 16 or 17 on every platform, and every other
///   profile only its own.
pub fn accepted_requirements(
    profile: SystemJavaProfileName,
    floor: Option<u8>,
) -> Vec<JavaRequirement> {
    if let Some(major) = floor_major(profile, floor) {
        return vec![JavaRequirement {
            or_later: true,
            ..requirement_for_major(profile, major)
        }];
    }

    let own = requirement_for(profile);
    let swapped = requirement_for_major(profile, effective_major(own.major, |_| false));

    if swapped == own {
        vec![own]
    } else {
        vec![own, swapped]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickReason {
    UserDefault,
    Managed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutomaticPick {
    Java { id: String, reason: PickReason },
    Download { major: u8 },
}

/// The Java Automatic uses for `req`:
/// 1. the user's default for the major, when that Java exists, is valid and
///    meets the requirement;
/// 2. otherwise the best valid managed Java that meets it: native to this
///    computer first, then one with no known issue (`flagged` holds the ids
///    that have one), then the newest;
/// 3. otherwise a download.
///
/// Found and user-added Javas are only ever used as a default.
pub fn pick_automatic(
    req: &JavaRequirement,
    javas: &[Java],
    defaults: &HashMap<u8, String>,
    host_arch: JavaArch,
    flagged: &HashSet<String>,
) -> AutomaticPick {
    let usable = |java: &&Java| java.is_valid && req.accepts_component(&java.component);

    if let Some(default_id) = defaults.get(&req.major)
        && javas
            .iter()
            .filter(usable)
            .any(|java| &java.id == default_id)
    {
        return AutomaticPick::Java {
            id: default_id.clone(),
            reason: PickReason::UserDefault,
        };
    }

    javas
        .iter()
        .filter(usable)
        .filter(|java| java.component._type == JavaComponentType::Managed)
        .filter_map(|java| Some((java, build_version(java)?)))
        .max_by(|(a, a_version), (b, b_version)| {
            let native = |java: &Java| java.component.arch == host_arch;
            let unflagged = |java: &Java| !flagged.contains(&java.id);

            native(a)
                .cmp(&native(b))
                .then_with(|| unflagged(a).cmp(&unflagged(b)))
                .then_with(|| a_version.cmp(b_version))
        })
        .map(|(java, _)| AutomaticPick::Java {
            id: java.id.clone(),
            reason: PickReason::Managed,
        })
        .unwrap_or(AutomaticPick::Download { major: req.major })
}

/// The Javas a fix may move an instance to, best first, with their majors:
/// valid ones `usable` takes (given each with its major), the oldest major
/// first, then GDLauncher-installed before found or user-added, native to
/// this computer before not, then newest. The crash fix and the fix before a
/// launch both rank by it.
pub fn fix_candidates_where(
    javas: &[Java],
    host_arch: JavaArch,
    usable: impl Fn(&Java, u8) -> bool,
) -> Vec<(String, u8)> {
    let mut candidates: Vec<(&Java, u8, JavaBuildVersion)> = javas
        .iter()
        .filter(|java| java.is_valid)
        .filter_map(|java| {
            let major = u8::try_from(java.component.version.major).ok()?;
            let version = build_version(java)?;
            usable(java, major).then_some((java, major, version))
        })
        .collect();

    candidates.sort_by(|(a, a_major, a_version), (b, b_major, b_version)| {
        let managed = |java: &Java| java.component._type == JavaComponentType::Managed;
        let native = |java: &Java| java.component.arch == host_arch;

        a_major
            .cmp(b_major)
            .then_with(|| managed(b).cmp(&managed(a)))
            .then_with(|| native(b).cmp(&native(a)))
            .then_with(|| b_version.cmp(a_version))
    });

    candidates
        .into_iter()
        .map(|(java, major, _)| (java.id.clone(), major))
        .collect()
}

/// Minecraft 1.17 asks for Java 16, which some platforms' catalogs (Linux
/// today) do not have: Java 17's choice serves it there.
pub fn effective_major(major: u8, catalog_has: impl Fn(u8) -> bool) -> u8 {
    if major == 16 && !catalog_has(16) {
        17
    } else {
        major
    }
}

/// Found and user-added Javas a launch may fall back to when the download
/// fails: valid, native, meeting one of `reqs` (earlier ones first), newest
/// first. Each comes with the requirement it meets.
pub fn fallback_candidates<'a>(
    reqs: &'a [JavaRequirement],
    javas: &[Java],
    host_arch: JavaArch,
) -> Vec<(String, &'a JavaRequirement)> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();

    for req in reqs {
        let mut matching: Vec<(&Java, JavaBuildVersion)> = javas
            .iter()
            .filter(|java| {
                java.is_valid
                    && matches!(
                        java.component._type,
                        JavaComponentType::Local | JavaComponentType::Custom
                    )
                    && java.component.arch == host_arch
                    && req.accepts_component(&java.component)
            })
            .filter_map(|java| Some((java, build_version(java)?)))
            .collect();
        matching.sort_by(|(_, a), (_, b)| b.cmp(a));

        for (java, _) in matching {
            if seen.insert(java.id.clone()) {
                out.push((java.id.clone(), req));
            }
        }
    }

    out
}

fn build_version(java: &Java) -> Option<JavaBuildVersion> {
    JavaBuildVersion::parse(&java.component.version.to_string())
}

/// This computer's CPU type as a Java arch. A host the launcher has no Java
/// builds for counts as x64, which only affects the native-first ordering.
pub fn host_arch() -> JavaArch {
    JavaArch::get_current_arch().unwrap_or(match std::env::consts::ARCH {
        "arm" => JavaArch::Arm32,
        _ => JavaArch::X86_64,
    })
}

/// Rows that cannot be read as a Java are left out.
pub(crate) fn javas_from_rows(rows: Vec<JavaRow>) -> Vec<Java> {
    rows.into_iter()
        .filter_map(|row| {
            let id = row.id.clone();
            Java::try_from(row)
                .inspect_err(|e| tracing::debug!("skipping unreadable java {id}: {e:#}"))
                .ok()
        })
        .collect()
}

/// Ids of the Javas a known issue affects on this computer, from the
/// advisories fetched this session.
pub(crate) async fn flagged_java_ids(app: &App, rows: &[JavaRow]) -> HashSet<String> {
    let manager = app.java_manager();
    let advisories = manager.advisories.read().await;

    match_advisories(rows, &advisories, &RealHostCpu)
        .into_iter()
        .map(|m| m.java_id)
        .collect()
}

/// The automatic choice for an instance that needs `profile`.
pub async fn resolve_automatic(
    app: &App,
    profile: SystemJavaProfileName,
) -> anyhow::Result<AutomaticPick> {
    Ok(resolve_automatic_requirement(app, profile, None).await?.1)
}

/// [`pick_automatic`] for an instance that needs `profile`, raised to its
/// Java `floor` (see [`need_requirement`]). When that ends in a download of a
/// major the download catalog lacks (`catalog_has`), the choice for
/// [`effective_major`] serves instead. Returns the requirement the pick
/// serves.
pub(crate) fn pick_for_profile(
    profile: SystemJavaProfileName,
    floor: Option<u8>,
    javas: &[Java],
    defaults: &HashMap<u8, String>,
    host_arch: JavaArch,
    flagged: &HashSet<String>,
    catalog_has: impl Fn(u8) -> bool,
) -> (JavaRequirement, AutomaticPick) {
    let req = need_requirement(profile, floor);
    let pick = pick_automatic(&req, javas, defaults, host_arch, flagged);

    let AutomaticPick::Download { major } = pick else {
        return (req, pick);
    };
    let effective = effective_major(major, catalog_has);
    if effective == major {
        return (req, pick);
    }

    let req = requirement_for_major(profile, effective);
    let pick = pick_automatic(&req, javas, defaults, host_arch, flagged);
    (req, pick)
}

/// [`resolve_automatic`] for an instance with the Java `floor`, plus the
/// requirement the pick serves, which differs from the profile's when Java
/// 17 stands in for Java 16 or the floor raises it.
pub(crate) async fn resolve_automatic_requirement(
    app: &App,
    profile: SystemJavaProfileName,
    floor: Option<u8>,
) -> anyhow::Result<(JavaRequirement, AutomaticPick)> {
    Ok(AutomaticInputs::load(app)
        .await?
        .pick(app, profile, floor)
        .await)
}

/// What the automatic choice is made from, read once so that many instances
/// resolve against the same state.
pub(crate) struct AutomaticInputs {
    pub javas: Vec<Java>,
    pub defaults: HashMap<u8, String>,
    /// Ids of the Javas with a known issue.
    pub flagged: HashSet<String>,
    pub host_arch: JavaArch,
    /// The majors the Azul catalog has, read on first need. `None` when it
    /// can't be reached, or isn't cached in cache-only mode.
    azul_majors: OnceCell<Option<HashSet<u8>>>,
    /// Read the catalog only from cache, never fetching it.
    cached_catalog_only: bool,
}

impl AutomaticInputs {
    pub fn new(
        javas: Vec<Java>,
        defaults: HashMap<u8, String>,
        flagged: HashSet<String>,
        host_arch: JavaArch,
    ) -> Self {
        Self {
            javas,
            defaults,
            flagged,
            host_arch,
            azul_majors: OnceCell::new(),
            cached_catalog_only: false,
        }
    }

    /// For what shows a resolution rather than launches: reads the Azul
    /// catalog only from cache, so it never waits on the network. Until the
    /// catalog has been fetched (the maintenance pass does at startup), no
    /// major counts as downloadable, so Java 17 stands in for 16.
    pub fn cached_catalog_only(mut self) -> Self {
        self.cached_catalog_only = true;
        self
    }

    pub async fn load(app: &App) -> anyhow::Result<Self> {
        let rows = java_repo::get_all_java(&app.db).await?;
        let flagged = flagged_java_ids(app, &rows).await;

        Ok(Self::new(
            javas_from_rows(rows),
            get_major_defaults(&app.db).await?,
            flagged,
            host_arch(),
        ))
    }

    /// Whether the Azul catalog has a build of `major`. Unreachable counts as
    /// missing: nothing could be downloaded anyway.
    pub async fn azul_has(&self, app: &App, major: u8) -> bool {
        self.azul_majors
            .get_or_init(|| async {
                let service = &app.java_manager().managed_service;
                let catalog = if self.cached_catalog_only {
                    service.cached_catalog(JavaVendor::Azul)
                } else {
                    service
                        .get_catalog(JavaVendor::Azul)
                        .await
                        .inspect_err(|e| tracing::debug!("could not read the Azul catalog: {e:#}"))
                        .ok()
                };

                catalog.map(|catalog| {
                    catalog
                        .majors
                        .iter()
                        .filter(|m| !m.builds.is_empty())
                        .map(|m| m.major)
                        .collect()
                })
            })
            .await
            .as_ref()
            .is_some_and(|majors| majors.contains(&major))
    }

    /// The automatic choice for an instance that needs `profile`, with the
    /// Java `floor`, and the requirement it serves (see [`pick_for_profile`]).
    pub async fn pick(
        &self,
        app: &App,
        profile: SystemJavaProfileName,
        floor: Option<u8>,
    ) -> (JavaRequirement, AutomaticPick) {
        let pick = |catalog_has: &dyn Fn(u8) -> bool| {
            pick_for_profile(
                profile,
                floor,
                &self.javas,
                &self.defaults,
                self.host_arch,
                &self.flagged,
                catalog_has,
            )
        };

        // Only a download of a major an empty catalog would swap can change
        // with the catalog, so it is fetched just for that.
        let (req, first) = pick(&|_| true);
        let AutomaticPick::Download { major } = first else {
            return (req, first);
        };
        if effective_major(major, |_| false) == major {
            return (req, first);
        }

        let has = self.azul_has(app, major).await;
        pick(&|m| m == major && has)
    }
}

/// Links `java_id` to every system profile whose requirement is `req`.
/// Returns whether any link changed.
pub(crate) async fn link_requirement_profiles(
    db: &Db,
    req: &JavaRequirement,
    java_id: &str,
) -> anyhow::Result<bool> {
    let names: HashSet<String> = SystemJavaProfileName::iter()
        .filter(|profile| requirement_for(*profile) == *req)
        .map(|profile| profile.to_string())
        .collect();

    let to_link: Vec<String> = java_repo::get_all_profiles(db)
        .await?
        .into_iter()
        .filter(|profile| {
            names.contains(&profile.name) && profile.java_id.as_deref() != Some(java_id)
        })
        .map(|profile| profile.name)
        .collect();

    if to_link.is_empty() {
        return Ok(false);
    }

    let java_id = java_id.to_string();
    // One writer dispatch and one transaction, so the profiles of a major
    // move together.
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        for name in to_link {
            java_repo::set_profile_java_conn(&tx, &name, Some(&java_id))?;
        }
        tx.commit()?;
        Ok(())
    })
    .await?;

    Ok(true)
}

/// Links every system profile to its automatic choice, after something that
/// changes it.
pub async fn relink_system_profiles(app: &App) -> anyhow::Result<()> {
    let rows = java_repo::get_all_java(&app.db).await?;
    let flagged = flagged_java_ids(app, &rows).await;

    sync_system_java_profiles(&app.db, &flagged).await?;
    invalidate_java_lists(app);

    Ok(())
}

/// The first fallback candidate (see [`fallback_candidates`]) that runs.
pub(crate) async fn fallback_java(
    app: &App,
    reqs: &[JavaRequirement],
) -> anyhow::Result<Option<JavaComponent>> {
    let javas = javas_from_rows(java_repo::get_all_java(&app.db).await?);

    for (id, req) in fallback_candidates(reqs, &javas, host_arch()) {
        if let Some(java) = app.java_manager().check_java_for(&id, req).await? {
            return Ok(Some(java));
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::java::{JavaComponent, JavaComponentType, JavaOs, JavaVersion};

    fn java(id: &str, ty: JavaComponentType, version: &str, arch: JavaArch, valid: bool) -> Java {
        Java {
            id: id.into(),
            component: JavaComponent {
                path: format!("/javas/{id}/bin/java"),
                arch,
                os: JavaOs::Linux,
                _type: ty,
                version: JavaVersion::try_from(version).unwrap(),
                vendor: "Azul Systems, Inc.".into(),
            },
            is_valid: valid,
        }
    }

    fn managed(id: &str, version: &str) -> Java {
        java(
            id,
            JavaComponentType::Managed,
            version,
            JavaArch::X86_64,
            true,
        )
    }

    fn pick(
        profile: SystemJavaProfileName,
        javas: &[Java],
        defaults: &[(u8, &str)],
        host_arch: JavaArch,
        flagged: &[&str],
    ) -> AutomaticPick {
        let defaults = defaults
            .iter()
            .map(|(major, id)| (*major, id.to_string()))
            .collect();
        let flagged = flagged.iter().map(|id| id.to_string()).collect();
        pick_automatic(
            &requirement_for(profile),
            javas,
            &defaults,
            host_arch,
            &flagged,
        )
    }

    fn picked(id: &str, reason: PickReason) -> AutomaticPick {
        AutomaticPick::Java {
            id: id.into(),
            reason,
        }
    }

    #[test]
    fn default_wins_when_valid() {
        let javas = [
            managed("m21", "21.0.13.1"),
            java(
                "c21",
                JavaComponentType::Custom,
                "21.0.5",
                JavaArch::X86_64,
                true,
            ),
        ];

        assert_eq!(
            pick(
                SystemJavaProfileName::Delta,
                &javas,
                &[(21, "c21")],
                JavaArch::X86_64,
                &[]
            ),
            picked("c21", PickReason::UserDefault)
        );
    }

    #[test]
    fn invalid_default_falls_back_to_managed() {
        let javas = [
            managed("m21", "21.0.13.1"),
            java(
                "c21",
                JavaComponentType::Custom,
                "21.0.5",
                JavaArch::X86_64,
                false,
            ),
            java(
                "c17",
                JavaComponentType::Custom,
                "17.0.5",
                JavaArch::X86_64,
                true,
            ),
        ];
        let delta = |default: &str| {
            pick(
                SystemJavaProfileName::Delta,
                &javas,
                &[(21, default)],
                JavaArch::X86_64,
                &[],
            )
        };

        // Invalid, deleted, or (after an in-place upgrade) of another major.
        assert_eq!(delta("c21"), picked("m21", PickReason::Managed));
        assert_eq!(delta("gone"), picked("m21", PickReason::Managed));
        assert_eq!(delta("c17"), picked("m21", PickReason::Managed));
    }

    #[test]
    fn found_java_is_never_picked_automatically() {
        let javas = [
            java(
                "l21",
                JavaComponentType::Local,
                "21.0.5",
                JavaArch::X86_64,
                true,
            ),
            java(
                "c21",
                JavaComponentType::Custom,
                "21.0.6",
                JavaArch::X86_64,
                true,
            ),
        ];

        assert_eq!(
            pick(
                SystemJavaProfileName::Delta,
                &javas,
                &[],
                JavaArch::X86_64,
                &[]
            ),
            AutomaticPick::Download { major: 21 }
        );
    }

    #[test]
    fn prefers_native_then_unflagged_then_newest() {
        let flagged_newest = [managed("new", "21.0.13.1"), managed("old", "21.0.12")];
        assert_eq!(
            pick(
                SystemJavaProfileName::Delta,
                &flagged_newest,
                &[],
                JavaArch::X86_64,
                &["new"]
            ),
            picked("old", PickReason::Managed)
        );
        assert_eq!(
            pick(
                SystemJavaProfileName::Delta,
                &flagged_newest,
                &[],
                JavaArch::X86_64,
                &[]
            ),
            picked("new", PickReason::Managed)
        );

        let mixed_arch = [
            java(
                "x64",
                JavaComponentType::Managed,
                "21.0.13.1",
                JavaArch::X86_64,
                true,
            ),
            java(
                "arm",
                JavaComponentType::Managed,
                "21.0.12",
                JavaArch::Arm64,
                true,
            ),
        ];
        assert_eq!(
            pick(
                SystemJavaProfileName::Delta,
                &mixed_arch,
                &[],
                JavaArch::Arm64,
                &[]
            ),
            picked("arm", PickReason::Managed)
        );
        // Native beats unflagged.
        assert_eq!(
            pick(
                SystemJavaProfileName::Delta,
                &mixed_arch,
                &[],
                JavaArch::Arm64,
                &["arm"]
            ),
            picked("arm", PickReason::Managed)
        );
    }

    #[test]
    fn legacy_fixed_1_respects_below() {
        assert_eq!(
            requirement_for(SystemJavaProfileName::LegacyFixed1),
            JavaRequirement {
                major: 8,
                below: JavaBuildVersion::parse("8.0.312"),
                or_later: false,
            }
        );

        let both = [managed("j492", "1.8.0_492"), managed("j302", "1.8.0_302")];
        assert_eq!(
            pick(
                SystemJavaProfileName::LegacyFixed1,
                &both,
                &[],
                JavaArch::X86_64,
                &[]
            ),
            picked("j302", PickReason::Managed)
        );
        assert_eq!(
            pick(
                SystemJavaProfileName::Legacy,
                &both,
                &[],
                JavaArch::X86_64,
                &[]
            ),
            picked("j492", PickReason::Managed)
        );

        let only_new = [managed("j492", "1.8.0_492")];
        assert_eq!(
            pick(
                SystemJavaProfileName::LegacyFixed1,
                &only_new,
                &[],
                JavaArch::X86_64,
                &[]
            ),
            AutomaticPick::Download { major: 8 }
        );
        // A default of 8u312 or later cannot serve it either.
        assert_eq!(
            pick(
                SystemJavaProfileName::LegacyFixed1,
                &only_new,
                &[(8, "j492")],
                JavaArch::X86_64,
                &[]
            ),
            AutomaticPick::Download { major: 8 }
        );
    }

    #[test]
    fn effective_profile_patches_only_forge_1_16_5() {
        use crate::domain::instance::info::{ModLoader, ModLoaderType, StandardVersion};

        let version = |release: &str, loader: Option<ModLoaderType>| StandardVersion {
            release: release.into(),
            modloaders: loader
                .map(|type_| ModLoader {
                    type_,
                    version: "1".into(),
                })
                .into_iter()
                .collect(),
        };
        let legacy = SystemJavaProfileName::Legacy;

        assert_eq!(
            effective_profile(legacy, &version("1.16.5", Some(ModLoaderType::Forge))),
            SystemJavaProfileName::LegacyFixed1
        );
        assert_eq!(effective_profile(legacy, &version("1.16.5", None)), legacy);
        assert_eq!(
            effective_profile(legacy, &version("1.16.5", Some(ModLoaderType::Fabric))),
            legacy
        );
        assert_eq!(
            effective_profile(legacy, &version("1.16.4", Some(ModLoaderType::Forge))),
            legacy
        );
    }

    #[test]
    fn override_choice_follows_the_launch_rules() {
        use crate::domain::instance::info::JavaOverride;

        let javas = [
            managed("m21", "21.0.13.1"),
            java(
                "broken",
                JavaComponentType::Custom,
                "21.0.5",
                JavaArch::X86_64,
                false,
            ),
        ];
        let profile = |name: String, java_id: Option<&str>, is_system| JavaProfile {
            name,
            java_id: java_id.map(str::to_string),
            is_system,
        };
        let system = |p: SystemJavaProfileName| p.to_string();
        let profiles = [
            profile("Mine".into(), Some("m21"), false),
            profile("Empty".into(), None, false),
            profile("Dangling".into(), Some("gone"), false),
            profile("Broken".into(), Some("broken"), false),
            profile(system(SystemJavaProfileName::Delta), Some("m21"), true),
            profile(system(SystemJavaProfileName::Epsilon), Some("broken"), true),
            profile(system(SystemJavaProfileName::Gamma), None, true),
        ];
        let required = Some(SystemJavaProfileName::Gamma);
        let managed_root = tempfile::tempdir().unwrap();
        let installed = managed_root.path().join("zulu21.0.5-a/bin/java");
        std::fs::create_dir_all(installed.parent().unwrap()).unwrap();
        std::fs::write(&installed, "").unwrap();
        let choose = |java_override: Option<JavaOverride>| {
            override_choice(
                java_override.as_ref(),
                required,
                &profiles,
                &javas,
                managed_root.path(),
            )
        };
        let at = |path: &std::path::Path| {
            Some(JavaOverride::Path(Some(
                path.to_string_lossy().into_owned(),
            )))
        };
        let named = |name: &str| Some(JavaOverride::Profile(Some(name.into())));
        let linked_m21 = OverrideChoice::Linked {
            id: "m21".into(),
            component: javas[0].component.clone(),
        };

        assert_eq!(choose(None), OverrideChoice::Automatic);
        assert_eq!(
            choose(Some(JavaOverride::Path(Some("/opt/java".into())))),
            OverrideChoice::Path("/opt/java".into())
        );
        // A GDLauncher Java that is gone is Automatic; one still there, or a
        // missing path elsewhere, is the path.
        assert_eq!(
            choose(at(&managed_root.path().join("zulu21.0.4-gone/bin/java"))),
            OverrideChoice::Automatic
        );
        assert_eq!(
            choose(at(&installed)),
            OverrideChoice::Path(installed.to_string_lossy().into_owned())
        );
        // The required system profile means Automatic.
        assert_eq!(
            choose(named(&system(SystemJavaProfileName::Gamma))),
            OverrideChoice::Automatic
        );
        // A profile that no longer exists falls back to Automatic, and so does
        // a custom profile whose Java is gone or doesn't start.
        for gone in ["Deleted", "Empty", "Dangling", "Broken"] {
            assert_eq!(choose(named(gone)), OverrideChoice::Automatic, "{gone}");
        }
        assert_eq!(choose(named("Mine")), linked_m21);
        assert_eq!(
            choose(named(&system(SystemJavaProfileName::Delta))),
            linked_m21
        );
        // Another system profile with nothing usable: its own automatic choice.
        assert_eq!(
            choose(named(&system(SystemJavaProfileName::Epsilon))),
            OverrideChoice::ProfileAutomatic(SystemJavaProfileName::Epsilon)
        );
        // Without a known requirement, even the instance's own profile is read
        // as a profile.
        assert_eq!(
            override_choice(
                named(&system(SystemJavaProfileName::Gamma)).as_ref(),
                None,
                &profiles,
                &javas,
                managed_root.path(),
            ),
            OverrideChoice::ProfileAutomatic(SystemJavaProfileName::Gamma)
        );

        // An override with nothing set (the old settings' Path or Profile
        // mode before a choice) is Automatic, as instance settings show it.
        for unset in [JavaOverride::Path(None), JavaOverride::Profile(None)] {
            assert_eq!(
                choose(Some(unset.clone())),
                OverrideChoice::Automatic,
                "{unset:?}"
            );
        }
    }

    #[test]
    fn accepted_requirements_take_java_16_and_17_for_1_17_only() {
        let majors = |profile| {
            accepted_requirements(profile, None)
                .iter()
                .map(|req| req.major)
                .collect::<Vec<_>>()
        };

        // Java 17 stands in for 16 wherever a catalog lacks it, so either fits.
        assert_eq!(majors(SystemJavaProfileName::Alpha), vec![16, 17]);
        for (profile, major) in [
            (SystemJavaProfileName::Legacy, 8),
            (SystemJavaProfileName::Gamma, 17),
            (SystemJavaProfileName::Delta, 21),
            (SystemJavaProfileName::Epsilon, 25),
        ] {
            assert_eq!(majors(profile), vec![major], "{profile:?}");
        }
        // Forge 1.16.5 keeps its cap.
        assert_eq!(
            accepted_requirements(SystemJavaProfileName::LegacyFixed1, None),
            vec![requirement_for(SystemJavaProfileName::LegacyFixed1)]
        );
    }

    /// A mod on Minecraft 1.20.1 (Java 17) crashed asking for Java 21.
    #[test]
    fn a_java_floor_takes_every_major_from_it_up() {
        let fits = |profile, floor, version: &str| {
            let java = managed("j", version);
            accepted_requirements(profile, floor)
                .iter()
                .any(|req| req.accepts_component(&java.component))
        };
        let gamma = SystemJavaProfileName::Gamma;

        assert!(!fits(gamma, Some(21), "17.0.13"));
        assert!(fits(gamma, Some(21), "21.0.5"));
        assert!(fits(gamma, Some(21), "25.0.1"));
        assert_eq!(
            accepted_requirements(gamma, Some(21)),
            vec![JavaRequirement {
                or_later: true,
                ..JavaRequirement::of_major(21)
            }]
        );
        // Never below what Minecraft needs.
        assert!(!fits(SystemJavaProfileName::Delta, Some(17), "17.0.13"));
        assert!(fits(SystemJavaProfileName::Delta, Some(17), "21.0.5"));
        // Forge 1.16.5's cap still holds on its own major.
        let forge = SystemJavaProfileName::LegacyFixed1;
        assert!(fits(forge, Some(8), "1.8.0_302"));
        assert!(!fits(forge, Some(8), "1.8.0_492"));
        assert!(fits(forge, Some(8), "11.0.20"));
    }

    #[test]
    fn automatic_picks_for_the_java_floor() {
        let javas = [
            managed("m17", "17.0.13"),
            managed("m21", "21.0.5"),
            managed("m25", "25.0.1"),
        ];
        let pick = |floor| {
            pick_for_profile(
                SystemJavaProfileName::Gamma,
                floor,
                &javas,
                &HashMap::new(),
                JavaArch::X86_64,
                &HashSet::new(),
                |_| true,
            )
        };

        assert_eq!(
            pick(None),
            (
                requirement_for(SystemJavaProfileName::Gamma),
                picked("m17", PickReason::Managed)
            )
        );
        assert_eq!(
            pick(Some(21)),
            (
                JavaRequirement::of_major(21),
                picked("m21", PickReason::Managed)
            )
        );
        // Nothing of the floor's major: it downloads that one.
        assert_eq!(
            pick(Some(22)),
            (
                JavaRequirement::of_major(22),
                AutomaticPick::Download { major: 22 }
            )
        );
    }

    #[test]
    fn effective_major_maps_16_to_17_only_when_missing() {
        assert_eq!(effective_major(16, |_| false), 17);
        assert_eq!(effective_major(16, |m| m == 16), 16);
        assert_eq!(effective_major(17, |_| false), 17);
        assert_eq!(effective_major(8, |_| false), 8);
    }

    #[test]
    fn java_16_uses_17s_choice_only_when_the_catalog_lacks_it() {
        let alpha = |javas: &[Java], defaults: &[(u8, &str)], catalog_has_16: bool| {
            let defaults = defaults
                .iter()
                .map(|(major, id)| (*major, id.to_string()))
                .collect();
            pick_for_profile(
                SystemJavaProfileName::Alpha,
                None,
                javas,
                &defaults,
                JavaArch::X86_64,
                &HashSet::new(),
                |major| catalog_has_16 && major == 16,
            )
        };
        let req = JavaRequirement::of_major;
        let only_17 = [managed("m17", "17.0.13")];

        // Linux: no Java 16 to download, so Java 17's choice serves 1.17.
        assert_eq!(
            alpha(&only_17, &[], false),
            (req(17), picked("m17", PickReason::Managed))
        );
        assert_eq!(
            alpha(&[], &[], false),
            (req(17), AutomaticPick::Download { major: 17 })
        );
        // Elsewhere Java 16 downloads as before.
        assert_eq!(
            alpha(&only_17, &[], true),
            (req(16), AutomaticPick::Download { major: 16 })
        );
        // A Java 16 the user has keeps serving it.
        let with_16 = [managed("m17", "17.0.13"), managed("m16", "16.0.2")];
        assert_eq!(
            alpha(&with_16, &[], false),
            (req(16), picked("m16", PickReason::Managed))
        );
        assert_eq!(
            alpha(&only_17, &[(17, "m17")], false),
            (req(17), picked("m17", PickReason::UserDefault))
        );
    }

    #[test]
    fn fallback_prefers_the_exact_major_then_the_newest_native_found_or_custom() {
        let javas = [
            java(
                "l17",
                JavaComponentType::Local,
                "17.0.5",
                JavaArch::X86_64,
                true,
            ),
            java(
                "c17",
                JavaComponentType::Custom,
                "17.0.9",
                JavaArch::X86_64,
                true,
            ),
            java(
                "l16",
                JavaComponentType::Local,
                "16.0.2",
                JavaArch::X86_64,
                true,
            ),
            java(
                "x86",
                JavaComponentType::Local,
                "16.0.2",
                JavaArch::X86_32,
                true,
            ),
            java(
                "bad",
                JavaComponentType::Custom,
                "16.0.3",
                JavaArch::X86_64,
                false,
            ),
            managed("m17", "17.0.13"),
        ];
        let reqs = [
            requirement_for(SystemJavaProfileName::Alpha),
            requirement_for(SystemJavaProfileName::Gamma),
        ];

        let ids: Vec<_> = fallback_candidates(&reqs, &javas, JavaArch::X86_64)
            .into_iter()
            .map(|(id, req)| (id, req.major))
            .collect();

        assert_eq!(
            ids,
            vec![
                ("l16".to_string(), 16),
                ("c17".to_string(), 17),
                ("l17".to_string(), 17)
            ]
        );
    }

    mod app {
        use super::*;
        use crate::managers::java::advisories::{AffectedRange, JavaAdvisory};
        use crate::setup_managers_for_test;

        fn row(id: &str, ty: &str, major: i32, version: &str) -> JavaRow {
            JavaRow {
                id: id.into(),
                path: format!("/nowhere/{id}/bin/java"),
                major,
                full_version: version.into(),
                r#type: ty.into(),
                os: "linux".into(),
                arch: host_arch().to_string(),
                vendor: "Azul Systems, Inc.".into(),
                is_valid: true,
            }
        }

        async fn linked(app: &App, profile: SystemJavaProfileName) -> Option<String> {
            java_repo::get_profile(&app.db, &profile.to_string())
                .await
                .unwrap()
                .unwrap()
                .java_id
        }

        #[tokio::test]
        async fn resolve_automatic_prefers_a_java_with_no_known_issue() {
            let app = setup_managers_for_test().await;
            for java in [
                row("new", "managed", 21, "21.0.13.1"),
                row("old", "managed", 21, "21.0.12"),
            ] {
                java_repo::insert_java(&app.db, java).await.unwrap();
            }

            assert_eq!(
                resolve_automatic(&app.app, SystemJavaProfileName::Delta)
                    .await
                    .unwrap(),
                picked("new", PickReason::Managed)
            );

            *app.java_manager().advisories.write().await = vec![JavaAdvisory {
                id: "issue".into(),
                name: "Issue".into(),
                severity: "high".into(),
                url: String::new(),
                affected: vec![AffectedRange {
                    major: 21,
                    introduced: Some("21.0.13".into()),
                    fixed: None,
                }],
                arch: vec![],
                cpu_features: vec![],
                mitigation_jvm_args: vec![],
            }];

            assert_eq!(
                resolve_automatic(&app.app, SystemJavaProfileName::Delta)
                    .await
                    .unwrap(),
                picked("old", PickReason::Managed)
            );
        }

        #[tokio::test]
        async fn resolve_automatic_serves_1_17_with_java_17_only_where_azul_has_no_16() {
            use crate::managers::java::managed::{
                BuildSource, ManagedJavaBuild, ManagedJavaCatalog,
            };

            let build = |id: &str, version: &[u32]| ManagedJavaBuild {
                id: id.into(),
                name: format!("{id}.zip"),
                major: version[0] as u8,
                version: JavaBuildVersion::from_components(version),
                source: BuildSource::AzulZip {
                    download_url: String::new(),
                    sha256: None,
                },
                size_bytes: None,
                release_date: None,
                lts: false,
            };
            let app = setup_managers_for_test().await;
            java_repo::insert_java(&app.db, row("m17", "managed", 17, "17.0.13"))
                .await
                .unwrap();
            let manager = app.java_manager();
            let service = &manager.managed_service;

            service
                .cache_azul_catalog_for_test(ManagedJavaCatalog::from_builds(vec![build(
                    "b17",
                    &[17, 0, 13],
                )]))
                .await;
            assert_eq!(
                resolve_automatic(&app.app, SystemJavaProfileName::Alpha)
                    .await
                    .unwrap(),
                picked("m17", PickReason::Managed)
            );

            service
                .cache_azul_catalog_for_test(ManagedJavaCatalog::from_builds(vec![
                    build("b17", &[17, 0, 13]),
                    build("b16", &[16, 0, 2]),
                ]))
                .await;
            assert_eq!(
                resolve_automatic(&app.app, SystemJavaProfileName::Alpha)
                    .await
                    .unwrap(),
                AutomaticPick::Download { major: 16 }
            );
        }

        #[tokio::test]
        async fn linking_a_requirement_links_only_its_profiles() {
            let app = setup_managers_for_test().await;
            java_repo::insert_java(&app.db, row("m17", "managed", 17, "17.0.13"))
                .await
                .unwrap();
            java_repo::insert_java(&app.db, row("m8", "managed", 8, "8.0.302"))
                .await
                .unwrap();

            let gamma = requirement_for(SystemJavaProfileName::Gamma);
            assert!(
                link_requirement_profiles(&app.db, &gamma, "m17")
                    .await
                    .unwrap()
            );
            assert!(
                !link_requirement_profiles(&app.db, &gamma, "m17")
                    .await
                    .unwrap()
            );
            let fixed = requirement_for(SystemJavaProfileName::LegacyFixed1);
            link_requirement_profiles(&app.db, &fixed, "m8")
                .await
                .unwrap();

            for profile in [
                SystemJavaProfileName::Beta,
                SystemJavaProfileName::Gamma,
                SystemJavaProfileName::GammaSnapshot,
            ] {
                assert_eq!(linked(&app.app, profile).await.as_deref(), Some("m17"));
            }
            assert_eq!(
                linked(&app.app, SystemJavaProfileName::LegacyFixed1)
                    .await
                    .as_deref(),
                Some("m8")
            );
            assert_eq!(linked(&app.app, SystemJavaProfileName::Legacy).await, None);
            assert_eq!(linked(&app.app, SystemJavaProfileName::Delta).await, None);
        }

        #[tokio::test]
        async fn a_picked_java_that_does_not_run_is_marked_invalid_and_unlinked() {
            let app = setup_managers_for_test().await;
            java_repo::insert_java(&app.db, row("m17", "managed", 17, "17.0.13"))
                .await
                .unwrap();
            let gamma = requirement_for(SystemJavaProfileName::Gamma);
            link_requirement_profiles(&app.db, &gamma, "m17")
                .await
                .unwrap();

            let checked = app
                .java_manager()
                .check_java_for("m17", &gamma)
                .await
                .unwrap();

            assert!(checked.is_none());
            assert!(
                !java_repo::get_java_by_id(&app.db, "m17")
                    .await
                    .unwrap()
                    .unwrap()
                    .is_valid
            );
            assert_eq!(linked(&app.app, SystemJavaProfileName::Gamma).await, None);
            // So the next resolve downloads instead.
            assert_eq!(
                resolve_automatic(&app.app, SystemJavaProfileName::Gamma)
                    .await
                    .unwrap(),
                AutomaticPick::Download { major: 17 }
            );
            assert!(
                app.java_manager()
                    .check_java_for("gone", &gamma)
                    .await
                    .unwrap()
                    .is_none()
            );
        }
    }
}
