//! The Java each instance launches with, worked out without running or
//! downloading anything. Instances resolve through the functions a launch
//! uses ([`override_choice`], [`effective_profile`] and
//! [`AutomaticInputs::pick`]), so the overview, the instance details and a
//! launch never disagree.

use super::{
    java_display::vendor_display_name,
    minecraft_majors::{
        MinecraftVersionProfiles, cached_version_profiles, minecraft_version_profiles,
    },
    resolve::{
        AutomaticInputs, AutomaticPick, JavaRequirement, OverrideChoice, PickReason,
        accepted_requirements, bounded_requirement, effective_profile, fix_candidates_where,
        floor_major, override_choice,
    },
};
use crate::{
    App,
    api::keys::{
        Key,
        java::{
            GET_AVAILABLE_JAVAS, GET_JAVA_ADVISORY_MATCHES, GET_JAVA_PROFILES,
            SYSTEM_JAVA_PROFILE_ASSIGNMENTS,
        },
    },
    domain::{
        instance::{
            InstanceId,
            info::{GameVersion, JavaOverride},
        },
        java::{
            Java, JavaComponent, JavaComponentType, JavaOs, JavaProfile,
            SYSTEM_JAVA_PROFILE_NAME_PREFIX, SystemJavaProfileName,
        },
    },
};
use anyhow::anyhow;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

/// The Java queries a resolution reads, besides the instance itself:
/// instance details go stale with them.
pub const RESOLUTION_SOURCES: [Key; 4] = [
    GET_AVAILABLE_JAVAS,
    GET_JAVA_PROFILES,
    SYSTEM_JAVA_PROFILE_ASSIGNMENTS,
    GET_JAVA_ADVISORY_MATCHES,
];

/// How an instance came to its Java.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaVia {
    /// Automatic picked one of GDLauncher's Javas, or downloads one.
    Automatic,
    /// Automatic picked the user's choice for the Java version.
    Default,
    /// The instance's Java path override.
    Instance,
    /// A profile the instance's override names.
    Profile,
}

impl JavaVia {
    /// Whether the instance itself names the Java.
    pub fn is_direct(self) -> bool {
        matches!(self, Self::Instance | Self::Profile)
    }
}

/// The Java an instance launches with, worked out without running or
/// downloading anything.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceJavaResolution {
    /// `None` for a download, or a path no listed Java has.
    pub java_id: Option<String>,
    pub path: Option<String>,
    /// The major of the Java it uses. `None` for a path no listed Java has,
    /// whose major only running it would tell.
    pub major: Option<u8>,
    pub via: JavaVia,
    /// Nothing is installed for Automatic to use, so a launch downloads it.
    pub downloads_at_launch: bool,
}

/// [`InstanceJavaResolution`] with the name the UI gives its Java.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedJava {
    pub resolution: InstanceJavaResolution,
    /// For a listed Java: its vendor's short name and its version.
    pub vendor_name: Option<String>,
    pub version: Option<String>,
}

/// What instance details say about the instance's Java.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InstanceJavaDetails {
    /// The system profile the instance's Minecraft version asks for. The
    /// frontend stores it as a `Profile` override to mean Automatic.
    pub required_profile: Option<SystemJavaProfileName>,
    /// The Java Automatic gives the instance: Forge 1.16.5's cap included
    /// (see [`effective_profile`]), Java 17 where it stands in for a Java 16
    /// the catalog lacks, and the Java floor's major when it has one. `None`
    /// when the version data lacks the release.
    pub need: Option<JavaRequirement>,
    /// Every Java the instance runs on ([`accepted_requirements`]). Empty when
    /// the version data lacks the release.
    pub accepted: Vec<JavaRequirement>,
    /// The Java floor's major, when a wrong-Java crash raised the instance's
    /// need (see [`floor_major`]): every Java from it up fits. `None` when the
    /// version data lacks the release.
    pub floor: Option<u8>,
    /// `None` when it can't be worked out, as a launch would fail to.
    pub resolved: Option<ResolvedJava>,
    /// What Automatic would give the instance, whatever its override says.
    /// `None` when it can't be worked out.
    pub automatic: Option<ResolvedJava>,
    /// The Java Automatic would download for the instance (`need`), while no
    /// valid Java that fits it is installed. `None` once one is, or without
    /// version data.
    pub download: Option<JavaRequirement>,
    /// The instance's override names a Java none of `accepted` takes.
    pub mismatch: bool,
    /// With `mismatch`, how the fix before a launch moves it onto a Java
    /// that fits.
    pub mismatch_fix: Option<MismatchFix>,
}

/// How the fix moves an instance off a Java that doesn't fit it, ranked as
/// the crash fix ranks (see [`fix_candidates_where`]), among the Javas that
/// fit: Automatic when it picks an installed Java, else an installed Java
/// that fits, else Automatic's download.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MismatchFix {
    /// The Java major the instance ends up on.
    pub major: u8,
    pub to: MismatchFixTo,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MismatchFixTo {
    /// No override: Automatic picks a Java that fits, or downloads one. With
    /// `floor`, the instance's Java floor rises to it, so Automatic picks
    /// the GDLauncher Java the fix found, of a major it wouldn't otherwise.
    Automatic { floor: Option<u8> },
    /// A found or user-added Java that fits (`req`), set in the instance.
    Java {
        id: String,
        path: String,
        req: JavaRequirement,
    },
}

/// An instance whose Java override names a Java its Minecraft version can't
/// run on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaMismatch {
    pub instance_id: InstanceId,
    pub need: JavaRequirement,
    /// Set when the instance's Java floor is what it needs: that major or
    /// later.
    pub floor: Option<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaUsage {
    pub instance_id: InstanceId,
    pub name: String,
    /// Set in the instance rather than through Automatic.
    pub direct: bool,
    /// Its requirement caps the build (Forge 1.16.5 needs Java 8 below update
    /// 312), so no newer build of the major serves it.
    pub bounded: bool,
    /// The custom profile it uses the Java through. Never one of the
    /// launcher's own profiles, whose names are codenames.
    pub via_profile: Option<String>,
}

/// The instance's Minecraft release isn't in the version data, so the Java
/// Automatic gives it is unknown.
#[derive(Debug, thiserror::Error)]
#[error("Can't tell which Java Minecraft {0} needs")]
pub(crate) struct NotInVersionData(String);

/// Why the profile an instance needs is unknown.
#[derive(Debug, Clone)]
enum UnknownNeed {
    NotInVersionData(String),
    /// Launches refuse custom versions, and instances without a version.
    NotLaunchable,
}

/// What instances resolve against, read once so they all see the same state.
pub(crate) struct ResolveContext {
    pub(crate) automatic: AutomaticInputs,
    profiles: Vec<JavaProfile>,
    /// The system profile each Minecraft version asks for, by version id.
    /// Empty without version data.
    required: HashMap<String, SystemJavaProfileName>,
    /// Where GDLauncher installs Javas.
    managed_root: PathBuf,
}

impl ResolveContext {
    /// For instance details, Java usage and mismatches, which show a
    /// resolution: they read the Java catalog only from cache.
    async fn load(app: &App) -> anyhow::Result<Self> {
        let automatic = AutomaticInputs::load(app).await?.cached_catalog_only();
        let versions = minecraft_version_profiles(app).await;

        Self::new(app, automatic, versions.as_deref()).await
    }

    /// [`load`](Self::load), with the version data it has however old
    /// (see [`cached_version_profiles`]): it never waits on the network.
    async fn load_cached(app: &App) -> anyhow::Result<Self> {
        let automatic = AutomaticInputs::load(app).await?.cached_catalog_only();
        let versions = cached_version_profiles(app).await;

        Self::new(app, automatic, versions.as_deref()).await
    }

    pub(crate) async fn new(
        app: &App,
        automatic: AutomaticInputs,
        versions: Option<&MinecraftVersionProfiles>,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            automatic,
            profiles: app.java_manager().get_java_profiles().await?,
            required: versions
                .map(MinecraftVersionProfiles::required)
                .unwrap_or_default(),
            managed_root: app
                .settings_manager()
                .runtime_path
                .get_managed_javas()
                .to_path(),
        })
    }

    /// The system profile `version` asks for, and the one it really needs
    /// (see [`effective_profile`]).
    fn profiles_for(
        &self,
        version: Option<&GameVersion>,
    ) -> Result<(SystemJavaProfileName, SystemJavaProfileName), UnknownNeed> {
        match version {
            Some(GameVersion::Standard(version)) => {
                let required = *self
                    .required
                    .get(&version.release)
                    .ok_or_else(|| UnknownNeed::NotInVersionData(version.release.clone()))?;
                Ok((required, effective_profile(required, version)))
            }
            Some(GameVersion::Custom(_)) | None => Err(UnknownNeed::NotLaunchable),
        }
    }

    /// In the order a launch applies them: a `Path` override, a `Profile`
    /// override, then Automatic, raised to the Java `floor`. Fails with
    /// [`NotInVersionData`] when it would need the release's requirement and
    /// the version data lacks it.
    async fn resolve(
        &self,
        app: &App,
        java_override: Option<&JavaOverride>,
        version: Option<&GameVersion>,
        floor: Option<u8>,
    ) -> anyhow::Result<InstanceJavaResolution> {
        let profiles = self.profiles_for(version);
        let needed = || {
            profiles
                .clone()
                .map(|(_, effective)| effective)
                .map_err(|need| match need {
                    UnknownNeed::NotInVersionData(release) => NotInVersionData(release).into(),
                    UnknownNeed::NotLaunchable => {
                        anyhow!("This instance has no Minecraft version a launch can start")
                    }
                })
        };
        let choice = override_choice(
            java_override,
            profiles.as_ref().ok().map(|(required, _)| *required),
            &self.profiles,
            &self.automatic.javas,
            &self.managed_root,
        );

        match choice {
            // A launch runs whatever is at the path, whatever the instance
            // needs, so this needs no version data.
            OverrideChoice::Path(path) => {
                let java = self
                    .automatic
                    .javas
                    .iter()
                    .find(|java| java.component.path == path);

                Ok(InstanceJavaResolution {
                    java_id: java.map(|java| java.id.clone()),
                    major: java.map(|java| java_major(&java.component)),
                    path: Some(path),
                    via: JavaVia::Instance,
                    downloads_at_launch: false,
                })
            }
            OverrideChoice::Linked { id, component } => Ok(InstanceJavaResolution {
                java_id: Some(id),
                major: Some(java_major(&component)),
                path: Some(component.path),
                via: JavaVia::Profile,
                downloads_at_launch: false,
            }),
            OverrideChoice::Automatic => Ok(self.automatic(app, needed()?, floor, false).await),
            OverrideChoice::ProfileAutomatic(profile) => {
                Ok(self.automatic(app, profile, None, true).await)
            }
        }
    }

    /// The automatic choice for `profile`, raised to the Java `floor`;
    /// `named` when the instance's override names that profile.
    async fn automatic(
        &self,
        app: &App,
        profile: SystemJavaProfileName,
        floor: Option<u8>,
        named: bool,
    ) -> InstanceJavaResolution {
        let (req, pick) = self.automatic.pick(app, profile, floor).await;
        let via = |reason| match reason {
            _ if named => JavaVia::Profile,
            Some(PickReason::UserDefault) => JavaVia::Default,
            _ => JavaVia::Automatic,
        };

        match pick {
            AutomaticPick::Java { id, reason } => InstanceJavaResolution {
                path: self
                    .automatic
                    .javas
                    .iter()
                    .find(|java| java.id == id)
                    .map(|java| java.component.path.clone()),
                java_id: Some(id),
                major: Some(req.major),
                via: via(Some(reason)),
                downloads_at_launch: false,
            },
            AutomaticPick::Download { major } => InstanceJavaResolution {
                java_id: None,
                path: None,
                major: Some(major),
                via: via(None),
                downloads_at_launch: true,
            },
        }
    }

    fn java(&self, java_id: &str) -> Option<&Java> {
        self.automatic.javas.iter().find(|java| java.id == java_id)
    }

    /// Whether `resolution` is a Java none of `accepted` takes. A path no
    /// listed Java has can't be told without running it, so it never is.
    fn misses(&self, accepted: &[JavaRequirement], resolution: &InstanceJavaResolution) -> bool {
        match resolution.java_id.as_deref().and_then(|id| self.java(id)) {
            Some(java) => !accepted
                .iter()
                .any(|req| req.accepts_component(&java.component)),
            None => resolution
                .major
                .is_some_and(|major| !accepted.iter().any(|req| req.accepts_major(major))),
        }
    }

    /// What instance details say about the Java of an instance with
    /// `java_override`, `version` and the Java `floor`.
    async fn details(
        &self,
        app: &App,
        java_override: Option<&JavaOverride>,
        version: Option<&GameVersion>,
        floor: Option<u8>,
    ) -> InstanceJavaDetails {
        let profiles = self.profiles_for(version).ok();
        // What Automatic serves the instance with, so the same as a launch.
        let need = match profiles {
            Some((_, effective)) => Some(self.automatic.pick(app, effective, floor).await.0),
            None => None,
        };
        // Whatever the catalog has, so it never flips with installs.
        let accepted = profiles
            .map(|(_, effective)| accepted_requirements(effective, floor))
            .unwrap_or_default();
        let resolution = self
            .resolve(app, java_override, version, floor)
            .await
            .inspect_err(|e| tracing::debug!("could not resolve an instance's Java: {e:#}"))
            .ok();
        let mismatch = java_override.is_some()
            && !accepted.is_empty()
            && resolution
                .as_ref()
                .is_some_and(|resolution| self.misses(&accepted, resolution));
        let automatic = match profiles {
            Some(_) => self.resolve(app, None, version, floor).await.ok(),
            None => None,
        };
        let fitting_installed = self.automatic.javas.iter().any(|java| {
            java.is_valid
                && accepted
                    .iter()
                    .any(|req| req.accepts_component(&java.component))
        });
        let download = need
            .clone()
            .filter(|_| !accepted.is_empty() && !fitting_installed);
        let mismatch_fix = match (mismatch, &automatic) {
            (true, Some(automatic)) => self.mismatch_fix(&accepted, automatic),
            _ => None,
        };

        InstanceJavaDetails {
            required_profile: profiles.map(|(required, _)| required),
            need,
            accepted,
            floor: profiles.and_then(|(_, effective)| floor_major(effective, floor)),
            resolved: resolution.map(|resolution| self.named(resolution)),
            automatic: automatic.map(|resolution| self.named(resolution)),
            download,
            mismatch,
            mismatch_fix,
        }
    }

    /// See [`MismatchFix`]. `automatic` is what Automatic gives the
    /// instance; `accepted` is what fits it.
    fn mismatch_fix(
        &self,
        accepted: &[JavaRequirement],
        automatic: &InstanceJavaResolution,
    ) -> Option<MismatchFix> {
        let to_automatic = |major| MismatchFix {
            major,
            to: MismatchFixTo::Automatic { floor: None },
        };
        let major = automatic.major?;
        if !automatic.downloads_at_launch {
            return Some(to_automatic(major));
        }

        let fits = |java: &Java| {
            accepted
                .iter()
                .find(|req| req.accepts_component(&java.component))
        };
        let candidates = fix_candidates_where(
            &self.automatic.javas,
            self.automatic.host_arch,
            |java, _| fits(java).is_some(),
        );
        let Some((java, fix_major)) = candidates
            .into_iter()
            .find_map(|(id, major)| Some((self.java(&id)?, major)))
        else {
            return Some(to_automatic(major));
        };

        Some(match java.component._type {
            // Automatic downloads, so it would pick a GDLauncher Java of its
            // own major: this one is of another major that fits.
            JavaComponentType::Managed => MismatchFix {
                major: fix_major,
                to: MismatchFixTo::Automatic {
                    floor: Some(fix_major),
                },
            },
            JavaComponentType::Local | JavaComponentType::Custom => MismatchFix {
                major: fix_major,
                to: MismatchFixTo::Java {
                    id: java.id.clone(),
                    path: java.component.path.clone(),
                    req: fits(java)?.clone(),
                },
            },
        })
    }

    /// `resolution` with the name the UI gives its Java.
    fn named(&self, resolution: InstanceJavaResolution) -> ResolvedJava {
        let java = resolution.java_id.as_deref().and_then(|id| self.java(id));

        ResolvedJava {
            vendor_name: java
                .map(|java| vendor_display_name(&java.component.vendor, &java.component.path)),
            version: java.map(|java| java.component.version.to_string()),
            resolution,
        }
    }
}

pub(crate) fn java_major(component: &JavaComponent) -> u8 {
    u8::try_from(component.version.major).unwrap_or(u8::MAX)
}

/// What the overview needs to know about an instance.
pub(crate) struct InstanceJava {
    pub(crate) id: InstanceId,
    pub(crate) name: String,
    pub(crate) java_override: Option<JavaOverride>,
    pub(crate) version: Option<GameVersion>,
    pub(crate) java_floor: Option<u8>,
    pub(crate) active: bool,
    /// Its game's process runs, spawned or adopted.
    pub(crate) running: bool,
    /// Its game was left running by an earlier session and adopted.
    pub(crate) adopted: bool,
}

pub(crate) async fn instance_javas(app: &App) -> Vec<InstanceJava> {
    let manager = app.instance_manager();
    let instances = manager.instances.read().await;

    instances
        .iter()
        .filter_map(|(id, instance)| {
            let data = instance.data().ok()?;
            Some(InstanceJava {
                id: *id,
                name: data.name().to_string(),
                java_override: data.java_override().cloned(),
                version: data.game_version().cloned(),
                java_floor: data.java_floor(),
                active: data.is_launch_active(),
                running: data.is_game_running(),
                adopted: data.is_adopted_game(),
            })
        })
        .collect()
}

/// Which instances use each Java.
pub(crate) struct Usage {
    pub(crate) by_java: HashMap<String, Vec<JavaUsage>>,
    /// The startup scan hasn't loaded the instances yet, or some instance's
    /// release isn't in the version data, so what some instance uses is
    /// unknown.
    pub(crate) incomplete: bool,
}

/// An instance whose Java can't be resolved, as a launch would fail to, uses
/// none.
pub(crate) async fn usage_with(
    app: &App,
    context: &ResolveContext,
    instances: &[InstanceJava],
) -> Usage {
    let mut by_java: HashMap<String, Vec<JavaUsage>> = HashMap::new();
    let mut incomplete = !*app.instance_manager().instances_scanned.borrow();

    for instance in instances {
        let resolved = context
            .resolve(
                app,
                instance.java_override.as_ref(),
                instance.version.as_ref(),
                instance.java_floor,
            )
            .await;

        match resolved {
            Ok(InstanceJavaResolution {
                java_id: Some(java_id),
                via,
                ..
            }) => {
                let bounded = context.bounded(instance.version.as_ref(), &java_id);
                let via_profile = match (via, &instance.java_override) {
                    (JavaVia::Profile, Some(JavaOverride::Profile(Some(name))))
                        if !name.starts_with(SYSTEM_JAVA_PROFILE_NAME_PREFIX) =>
                    {
                        Some(name.clone())
                    }
                    _ => None,
                };
                by_java.entry(java_id).or_default().push(JavaUsage {
                    instance_id: instance.id,
                    name: instance.name.clone(),
                    direct: via.is_direct(),
                    bounded,
                    via_profile,
                });
            }
            Ok(_) => {}
            Err(e) => {
                incomplete |= e.downcast_ref::<NotInVersionData>().is_some();
                tracing::debug!(
                    "could not resolve the Java of instance {}: {e:#}",
                    instance.id.0
                );
            }
        }
    }

    for users in by_java.values_mut() {
        users.sort_by(|a, b| {
            a.name
                .to_lowercase()
                .cmp(&b.name.to_lowercase())
                .then(a.instance_id.0.cmp(&b.instance_id.0))
        });
    }

    Usage {
        by_java,
        incomplete,
    }
}

impl ResolveContext {
    /// How many of `instances` name each custom profile in their Java
    /// override, whether or not its Java is still there. Deleting the profile
    /// moves them to Automatic, as an empty profile already does (see
    /// [`override_choice`]).
    pub(crate) fn custom_profile_users(&self, instances: &[InstanceJava]) -> HashMap<String, u32> {
        let mut users: HashMap<String, u32> = HashMap::new();

        for instance in instances {
            let Some(JavaOverride::Profile(Some(name))) = &instance.java_override else {
                continue;
            };
            if self
                .profiles
                .iter()
                .any(|profile| !profile.is_system && profile.name == *name)
            {
                *users.entry(name.clone()).or_default() += 1;
            }
        }

        users
    }

    /// Whether an instance of `version` needs the Java `java_id` because of a
    /// cap on the build ([`bounded_requirement`]) that the Java meets.
    fn bounded(&self, version: Option<&GameVersion>, java_id: &str) -> bool {
        let Some(GameVersion::Standard(version)) = version else {
            return false;
        };
        let java = self.automatic.javas.iter().find(|java| java.id == java_id);

        bounded_requirement(version)
            .zip(java)
            .is_some_and(|(req, java)| req.accepts_component(&java.component))
    }
}

/// The Java `instance_id` launches with, worked out the way a launch does it
/// but without checking or downloading anything.
pub async fn resolve_instance_java(
    app: &App,
    instance_id: InstanceId,
) -> anyhow::Result<InstanceJavaResolution> {
    let (java_override, version, floor) = {
        let manager = app.instance_manager();
        let instances = manager.instances.read().await;
        let data = instances
            .get(&instance_id)
            .ok_or_else(|| anyhow!("Instance {} not found", instance_id.0))?
            .data()?;

        (
            data.java_override().cloned(),
            data.game_version().cloned(),
            data.java_floor(),
        )
    };

    ResolveContext::load(app)
        .await?
        .resolve(app, java_override.as_ref(), version.as_ref(), floor)
        .await
}

/// What instance details say about the Java of an instance with
/// `java_override`, `version` and the Java `floor`, resolved as by
/// [`resolve_instance_java`]. Takes the instance's settings rather than its
/// id, so a caller holding the instances lock can pass them.
pub async fn instance_java_details(
    app: &App,
    java_override: Option<&JavaOverride>,
    version: Option<&GameVersion>,
    floor: Option<u8>,
) -> anyhow::Result<InstanceJavaDetails> {
    Ok(ResolveContext::load(app)
        .await?
        .details(app, java_override, version, floor)
        .await)
}

/// [`instance_java_details`] from the version data there is, however old:
/// for the check before a launch, which must never wait on the network.
pub async fn instance_java_details_cached(
    app: &App,
    java_override: Option<&JavaOverride>,
    version: Option<&GameVersion>,
    floor: Option<u8>,
) -> anyhow::Result<InstanceJavaDetails> {
    Ok(ResolveContext::load_cached(app)
        .await?
        .details(app, java_override, version, floor)
        .await)
}

/// The instances whose override names a Java their Minecraft version can't
/// run on, as their details report it. Best effort, for the library's tiles:
/// when the Javas can't be read, it logs that and reports none.
pub async fn java_mismatches(app: &App) -> Vec<JavaMismatch> {
    let overridden: Vec<InstanceJava> = instance_javas(app)
        .await
        .into_iter()
        .filter(|instance| instance.java_override.is_some())
        .collect();
    // Most libraries have none, and then nothing needs loading.
    if overridden.is_empty() {
        return Vec::new();
    }

    let context = match ResolveContext::load(app).await {
        Ok(context) => context,
        Err(e) => {
            tracing::warn!("could not check the instances' Javas: {e:#}");
            return Vec::new();
        }
    };
    let mut mismatches = Vec::new();
    for instance in overridden {
        let details = context
            .details(
                app,
                instance.java_override.as_ref(),
                instance.version.as_ref(),
                instance.java_floor,
            )
            .await;
        if let (true, Some(need)) = (details.mismatch, details.need) {
            mismatches.push(JavaMismatch {
                instance_id: instance.id,
                need,
                floor: details.floor,
            });
        }
    }

    mismatches
}

/// The Javas the games starting or running, and the servers running, may
/// run on.
#[derive(Debug)]
pub(crate) struct RunningJavas {
    ids: HashSet<String>,
    /// The folders of the binaries they run (see [`java_folder_key`]), as
    /// written and resolved, so a path spelled through a link still matches.
    folders: HashSet<String>,
    /// What one of them runs on is unknown, so every Java counts.
    unknown: bool,
    /// The OS the paths are of.
    os: JavaOs,
}

impl Default for RunningJavas {
    fn default() -> Self {
        Self::new(JavaOs::get_current_os().unwrap_or(JavaOs::Linux))
    }
}

impl RunningJavas {
    fn new(os: JavaOs) -> Self {
        Self {
            ids: HashSet::new(),
            folders: HashSet::new(),
            unknown: false,
            os,
        }
    }

    /// Whether a running game or server may run on the Java `id` at `path`.
    pub(crate) fn uses(&self, id: &str, path: &str) -> bool {
        self.unknown
            || self.ids.contains(id)
            || self
                .folder_keys(path)
                .iter()
                .any(|key| self.folders.contains(key))
    }

    /// Only the binary at `path` runs.
    fn of_binary(path: String) -> Self {
        let mut running = Self::default();
        running.add_path(path);
        running
    }

    /// A binary something runs.
    fn add_path(&mut self, path: String) {
        let keys = self.folder_keys(&path);
        self.folders.extend(keys);
    }

    /// The folder of the binary at `path`, as written and resolved.
    fn folder_keys(&self, path: &str) -> Vec<String> {
        let resolved = std::fs::canonicalize(path)
            .ok()
            .map(|resolved| resolved.to_string_lossy().into_owned());

        [Some(path.to_string()), resolved]
            .into_iter()
            .flatten()
            .filter_map(|path| java_folder_key(&path, self.os))
            .collect()
    }
}

/// The folder a Java binary sits in, which tells the Java it belongs to: a
/// game on Windows runs the `javaw.exe` next to the `java.exe` its Java is
/// listed by. On Windows it ignores case and takes either separator. Works
/// on the string alone, so it reads a Windows path on any OS.
pub(crate) fn java_folder_key(binary: &str, os: JavaOs) -> Option<String> {
    match os {
        JavaOs::Windows => {
            let binary = binary.replace('/', "\\");
            let folder = &binary[..binary.rfind('\\')?];
            Some(folder.to_lowercase())
        }
        JavaOs::Linux | JavaOs::MacOs => Some(binary[..binary.rfind('/')?].to_string()),
    }
}

/// What the games starting or running and the running servers may run on:
/// each game's Java resolved as a launch does it (`versions` is the version
/// data, read beforehand since it may wait on the network), plus the Java
/// its launch started on, which it keeps when its instance moves meanwhile;
/// each server's Java. A game adopted from an earlier session whose Java the
/// process table didn't tell may run on any of them, and so may anything
/// before the startup scan has adopted those games.
pub(crate) async fn running_javas(
    app: &App,
    versions: Option<&MinecraftVersionProfiles>,
) -> anyhow::Result<RunningJavas> {
    if !*app.instance_manager().instances_scanned.borrow() {
        return Ok(RunningJavas {
            unknown: true,
            ..Default::default()
        });
    }

    let mut running = RunningJavas::default();
    let games: Vec<InstanceJava> = instance_javas(app)
        .await
        .into_iter()
        .filter(|instance| instance.active)
        .collect();

    if !games.is_empty() {
        let automatic = AutomaticInputs::load(app).await?.cached_catalog_only();
        let context = ResolveContext::new(app, automatic, versions).await?;

        for game in games {
            match app.java_manager().launch_java(game.id) {
                Some(path) => running.add_path(path),
                // Its instance may have moved to another Java since that
                // session launched it.
                None if game.adopted => running.unknown = true,
                None => {}
            }
            let resolved = context
                .resolve(
                    app,
                    game.java_override.as_ref(),
                    game.version.as_ref(),
                    game.java_floor,
                )
                .await;
            match resolved {
                Ok(resolution) => {
                    running.ids.extend(resolution.java_id);
                    if let Some(path) = resolution.path {
                        running.add_path(path);
                    }
                }
                Err(e) => {
                    tracing::debug!("could not tell the Java game {} runs on: {e:#}", game.id.0);
                    running.unknown = true;
                }
            }
        }
    }

    for server in app.server_manager().active_server_javas().await {
        match server {
            Some(path) => running.add_path(path.to_string_lossy().into_owned()),
            None => running.unknown = true,
        }
    }

    Ok(running)
}

/// The games starting or running, by instance id, each with the Java its
/// process runs on when this session knows it: the one its launch started
/// on, or the one adoption read, matched by folder as [`running_javas`]
/// does. It keeps that Java when its instance moves to another. `None`
/// while the game is starting, and for a game adopted without its Java: its
/// instance's Java stands in.
pub(crate) fn active_games(
    app: &App,
    instances: &[InstanceJava],
) -> HashMap<i32, Option<RunningJavas>> {
    instances
        .iter()
        .filter(|instance| instance.active)
        .map(|instance| {
            let launched = instance
                .running
                .then(|| app.java_manager().launch_java(instance.id))
                .flatten()
                .map(RunningJavas::of_binary);
            (instance.id.0, launched)
        })
        .collect()
}

/// The instances that use each Java, by Java id, each resolved as by
/// [`resolve_instance_java`].
pub async fn java_usage(app: &App) -> anyhow::Result<HashMap<String, Vec<JavaUsage>>> {
    let context = ResolveContext::load(app).await?;

    Ok(usage_with(app, &context, &instance_javas(app).await)
        .await
        .by_java)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::managers::java::test_support::*;

    #[tokio::test]
    async fn resolve_instance_java_matches_the_launch_rules() {
        let app = test_app(vec![build("b17", &[17, 0, 13])]).await;
        add_managed(&app.app, "m8-new", "zulu8.0.492-a", "8.0.492", 1).await;
        add_managed(&app.app, "m8-old", "zulu8.0.302-b", "8.0.302", 1).await;
        add_managed(&app.app, "m17", "zulu17.0.13-c", "17.0.13", 1).await;
        let required =
            |profile: SystemJavaProfileName| Some(JavaOverride::Profile(Some(profile.to_string())));
        let resolve = |id| {
            let app = app.app.clone();
            async move { resolve_instance_java(&app, id).await.unwrap() }
        };

        // Forge 1.16.5 needs Java 8 below update 312.
        let forge = add_instance(&app.app, "Forge", "1.16.5", true, None).await;
        let vanilla = add_instance(&app.app, "Vanilla", "1.16.5", false, None).await;
        let forge_required = add_instance(
            &app.app,
            "ForgeReq",
            "1.16.5",
            true,
            required(SystemJavaProfileName::Legacy),
        )
        .await;
        for id in [forge, forge_required] {
            assert_eq!(
                resolve(id).await,
                InstanceJavaResolution {
                    java_id: Some("m8-old".into()),
                    path: Some(entry_path(&app.app, "m8-old").await),
                    major: Some(8),
                    via: JavaVia::Automatic,
                    downloads_at_launch: false,
                }
            );
        }
        assert_eq!(resolve(vanilla).await.java_id.as_deref(), Some("m8-new"));

        // 1.17 on a platform whose catalog has no Java 16: Java 17's choice.
        let java16 = add_instance(&app.app, "Caves", "1.17", false, None).await;
        let resolved = resolve(java16).await;
        assert_eq!(
            (resolved.java_id.as_deref(), resolved.major),
            (Some("m17"), Some(17))
        );

        // Nothing for Java 21: a launch downloads it.
        let delta = add_instance(&app.app, "Delta", "1.21.4", false, None).await;
        assert_eq!(
            resolve(delta).await,
            InstanceJavaResolution {
                java_id: None,
                path: None,
                major: Some(21),
                via: JavaVia::Automatic,
                downloads_at_launch: true,
            }
        );

        // The user's choice for the major.
        crate::managers::java::major_defaults::set_major_default(&app.db, 8, Some("m8-old".into()))
            .await
            .unwrap();
        assert_eq!(resolve(vanilla).await.via, JavaVia::Default);

        // A path no listed Java has: its major is unknown without running it.
        let unknown = add_instance(
            &app.app,
            "Unknown",
            "1.21.4",
            false,
            Some(JavaOverride::Path(Some("/opt/somewhere/bin/java".into()))),
        )
        .await;
        assert_eq!(
            resolve(unknown).await,
            InstanceJavaResolution {
                java_id: None,
                path: Some("/opt/somewhere/bin/java".into()),
                major: None,
                via: JavaVia::Instance,
                downloads_at_launch: false,
            }
        );

        // A custom profile.
        app.java_manager()
            .create_java_profile("Mine".into(), Some("m17".into()))
            .await
            .unwrap();
        let profiled = add_instance(
            &app.app,
            "Profiled",
            "1.21.4",
            false,
            Some(JavaOverride::Profile(Some("Mine".into()))),
        )
        .await;
        let resolved = resolve(profiled).await;
        assert_eq!(
            (resolved.java_id.as_deref(), resolved.major, resolved.via),
            (Some("m17"), Some(17), JavaVia::Profile)
        );
        // An old setting naming one of the launcher's own profiles, not the
        // one its version needs: Automatic's choice for that profile.
        let named_system = add_instance(
            &app.app,
            "Named",
            "1.21.4",
            false,
            required(SystemJavaProfileName::Gamma),
        )
        .await;
        assert_eq!(resolve(named_system).await.via, JavaVia::Profile);

        // Usage agrees, and names how each instance got there.
        let usage = java_usage(&app.app).await.unwrap();
        assert_eq!(
            usage["m8-old"],
            vec![
                super::JavaUsage {
                    instance_id: forge,
                    name: "Forge".into(),
                    direct: false,
                    bounded: true,
                    via_profile: None,
                },
                super::JavaUsage {
                    instance_id: forge_required,
                    name: "ForgeReq".into(),
                    direct: false,
                    bounded: true,
                    via_profile: None,
                },
                super::JavaUsage {
                    instance_id: vanilla,
                    name: "Vanilla".into(),
                    direct: false,
                    bounded: false,
                    via_profile: None,
                },
            ]
        );
        // A custom profile is named; the launcher's own profiles never are.
        assert_eq!(
            usage["m17"],
            vec![
                usage_of("Caves", java16, false),
                usage_of("Named", named_system, true),
                JavaUsage {
                    via_profile: Some("Mine".into()),
                    ..usage_of("Profiled", profiled, true)
                },
            ]
        );
        assert!(!usage.contains_key("m8-new"));
    }

    fn usage_of(name: &str, id: InstanceId, direct: bool) -> JavaUsage {
        usage(name, id, direct)
    }

    fn path(row: &JavaRow) -> Option<JavaOverride> {
        Some(JavaOverride::Path(Some(row.path.clone())))
    }

    #[tokio::test]
    async fn instance_details_resolved_java_matches_launch() {
        use crate::managers::java::resolve::{AutomaticPick, requirement_for, resolve_automatic};

        let app = test_app(vec![build("b21", &[21, 0, 13, 1])]).await;
        let m21 = add_managed(&app.app, "m21", "zulu21.0.13.1-a", "21.0.13.1", 1).await;
        let m8 = add_managed(&app.app, "m8-new", "zulu8.0.492-b", "8.0.492", 1).await;
        let details = |id| {
            let app = app.app.clone();
            async move { app.instance_manager().instance_details(id).await.unwrap() }
        };

        // Automatic on 1.21.4: what a launch picks.
        let auto = add_instance(&app.app, "Auto", "1.21.4", false, None).await;
        let AutomaticPick::Java { id: picked, .. } =
            resolve_automatic(&app.app, SystemJavaProfileName::Delta)
                .await
                .unwrap()
        else {
            panic!("a launch would download Java 21");
        };
        let auto = details(auto).await;
        let resolved = auto.resolved_java.unwrap();
        assert_eq!(resolved.resolution.java_id, Some(picked));
        assert_eq!(resolved.resolution.via, JavaVia::Automatic);
        assert_eq!(resolved.resolution.path, Some(m21.path.clone()));
        assert_eq!(resolved.vendor_name.as_deref(), Some("Azul Zulu"));
        assert_eq!(resolved.version.as_deref(), Some("21.0.13.1"));
        assert_eq!(auto.required_java.map(|req| req.major), Some(21));
        assert_eq!(
            auto.required_java_profile,
            Some(SystemJavaProfileName::Delta.to_string())
        );
        assert!(!auto.java_override_mismatch);

        // Forge 1.16.5 needs Java 8 below update 312, so 8.0.492 is the wrong
        // Java for it, though it serves vanilla 1.16.5.
        let forge = add_instance(&app.app, "Forge", "1.16.5", true, path(&m8)).await;
        let forge = details(forge).await;
        assert_eq!(
            forge.required_java,
            Some(requirement_for(SystemJavaProfileName::LegacyFixed1))
        );
        assert_eq!(forge.required_java.map(|req| req.major), Some(8));
        assert_eq!(forge.resolved_java.unwrap().resolution.major, Some(8));
        assert!(forge.java_override_mismatch);
        let vanilla = add_instance(&app.app, "Vanilla", "1.16.5", false, path(&m8)).await;
        assert!(!details(vanilla).await.java_override_mismatch);

        // Another major is the wrong Java; the required profile is Automatic.
        let wrong = add_instance(&app.app, "Wrong", "1.16.5", false, path(&m21)).await;
        assert!(details(wrong).await.java_override_mismatch);
        let required = add_instance(
            &app.app,
            "Required",
            "1.21.4",
            false,
            Some(JavaOverride::Profile(Some(
                SystemJavaProfileName::Delta.to_string(),
            ))),
        )
        .await;
        let required = details(required).await;
        assert_eq!(
            required.resolved_java.unwrap().resolution.via,
            JavaVia::Automatic
        );
        assert!(!required.java_override_mismatch);
    }

    #[tokio::test]
    async fn java_16_and_17_both_fit_minecraft_1_17() {
        // The catalog has no Java 16, as on Linux.
        let app = test_app(vec![build("b17", &[17, 0, 13])]).await;
        let m17 = add_managed(&app.app, "m17", "zulu17.0.13-a", "17.0.13", 1).await;
        let l16 = add_other(
            &app.app,
            "l16",
            "local",
            "/usr/lib/jvm/java-16/bin/java",
            "16.0.2",
        )
        .await;
        let l11 = add_other(
            &app.app,
            "l11",
            "local",
            "/usr/lib/jvm/java-11/bin/java",
            "11.0.20",
        )
        .await;
        let details = |id| {
            let app = app.app.clone();
            async move { app.instance_manager().instance_details(id).await.unwrap() }
        };
        let majors =
            |reqs: &[JavaRequirement]| reqs.iter().map(|req| req.major).collect::<Vec<_>>();

        let auto = add_instance(&app.app, "Auto", "1.17", false, None).await;
        let on16 = add_instance(&app.app, "On16", "1.17", false, path(&l16)).await;
        let on17 = add_instance(&app.app, "On17", "1.17", false, path(&m17)).await;
        let on11 = add_instance(&app.app, "On11", "1.17", false, path(&l11)).await;

        // Automatic downloads nothing for 16 here, so Java 17 is what it needs.
        let auto = details(auto).await;
        assert_eq!(auto.required_java.map(|req| req.major), Some(17));
        assert_eq!(majors(&auto.accepted_java), vec![16, 17]);
        assert!(!details(on16).await.java_override_mismatch);
        assert!(!details(on17).await.java_override_mismatch);
        assert!(details(on11).await.java_override_mismatch);

        // With a GDLauncher Java 16, Automatic needs 16, and 17 still fits.
        add_managed(&app.app, "m16", "zulu16.0.2-b", "16.0.2", 1).await;
        let on17 = details(on17).await;
        assert_eq!(on17.required_java.map(|req| req.major), Some(16));
        assert!(!on17.java_override_mismatch);

        let mismatched: Vec<_> = java_mismatches(&app.app)
            .await
            .into_iter()
            .map(|m| m.instance_id)
            .collect();
        assert_eq!(mismatched, vec![on11]);
    }

    /// The dropdown's Automatic option shows what switching back gives, so
    /// the details say it whatever the override.
    #[tokio::test]
    async fn details_say_what_automatic_would_pick_whatever_the_override() {
        let app = test_app(vec![build("b21", &[21, 0, 13, 1])]).await;
        add_managed(&app.app, "m21", "zulu21.0.13.1-a", "21.0.13.1", 1).await;
        let l17 = add_other(
            &app.app,
            "l17",
            "local",
            "/usr/lib/jvm/java-17/bin/java",
            "17.0.12",
        )
        .await;
        let details = |id| {
            let app = app.app.clone();
            async move { app.instance_manager().instance_details(id).await.unwrap() }
        };

        let set = add_instance(&app.app, "Set", "1.21.4", false, path(&l17)).await;
        let set = details(set).await;
        assert_eq!(
            set.resolved_java.unwrap().resolution.java_id.as_deref(),
            Some("l17")
        );
        let automatic = set.automatic_java.unwrap();
        assert_eq!(
            (
                automatic.resolution.java_id.as_deref(),
                automatic.resolution.major,
                automatic.resolution.via,
                automatic.vendor_name.as_deref(),
                automatic.version.as_deref(),
            ),
            (
                Some("m21"),
                Some(21),
                JavaVia::Automatic,
                Some("Azul Zulu"),
                Some("21.0.13.1")
            )
        );

        // Nothing for Java 25: it downloads at launch.
        let newest = add_instance(&app.app, "Newest", "26.1", false, path(&l17)).await;
        let automatic = details(newest).await.automatic_java.unwrap().resolution;
        assert_eq!(
            (automatic.major, automatic.downloads_at_launch),
            (Some(25), true)
        );
    }

    /// "Download Java N…" offers the Java Automatic would download, only
    /// while no Java that fits is installed.
    #[tokio::test]
    async fn details_say_which_java_automatic_would_download() {
        // As on Linux: no Java 16 to download.
        let app = test_app(vec![build("b17", &[17, 0, 13]), build("b8", &[8, 0, 302])]).await;
        let l11 = add_other(
            &app.app,
            "l11",
            "local",
            "/usr/lib/jvm/java-11/bin/java",
            "11.0.20",
        )
        .await;
        let details = |id| {
            let app = app.app.clone();
            async move { app.instance_manager().instance_details(id).await.unwrap() }
        };
        let download = |details: &crate::domain::instance::InstanceDetails| {
            details
                .download_java
                .as_ref()
                .map(|req| (req.major, req.below.as_ref().map(ToString::to_string)))
        };

        // 1.17 asks for Java 16, which can't be downloaded here: 17.
        let caves = add_instance(&app.app, "Caves", "1.17", false, path(&l11)).await;
        assert_eq!(download(&details(caves).await), Some((17, None)));

        // Forge 1.16.5 needs Java 8 below update 312.
        let forge = add_instance(&app.app, "Forge", "1.16.5", true, None).await;
        assert_eq!(
            download(&details(forge).await),
            Some((8, Some("8.0.312".to_string())))
        );

        // A found Java that fits is installed: nothing to offer.
        add_other(
            &app.app,
            "l16",
            "local",
            "/usr/lib/jvm/java-16/bin/java",
            "16.0.2",
        )
        .await;
        assert_eq!(download(&details(caves).await), None);

        // A Java 8 at update 312 or later doesn't fit Forge 1.16.5.
        add_other(&app.app, "l8", "local", "/opt/jdk8/bin/java", "8.0.492").await;
        assert_eq!(
            download(&details(forge).await),
            Some((8, Some("8.0.312".to_string())))
        );
    }

    /// Before a launch on a Java that doesn't fit, the fix moves the
    /// instance as the crash fix would: Automatic when it has a Java, else an
    /// installed Java that fits, else Automatic's download. Never onto a Java
    /// that doesn't fit, however new.
    #[tokio::test]
    async fn a_mismatch_is_fixed_with_a_java_that_fits() {
        let app = test_app(vec![
            build("b21", &[21, 0, 13, 1]),
            build("b16", &[16, 0, 2]),
        ])
        .await;
        let l17 = add_other(
            &app.app,
            "l17",
            "local",
            "/usr/lib/jvm/java-17/bin/java",
            "17.0.12",
        )
        .await;
        let l11 = add_other(
            &app.app,
            "l11",
            "local",
            "/usr/lib/jvm/java-11/bin/java",
            "11.0.20",
        )
        .await;
        let fix = |id| {
            let app = app.app.clone();
            async move {
                app.instance_manager()
                    .instance_details(id)
                    .await
                    .unwrap()
                    .java_mismatch_fix
            }
        };
        let automatic = |major, floor| {
            Some(MismatchFix {
                major,
                to: MismatchFixTo::Automatic { floor },
            })
        };

        // Nothing for Java 21: Automatic downloads it.
        let on17 = add_instance(&app.app, "On17", "1.21.4", false, path(&l17)).await;
        assert_eq!(fix(on17).await, automatic(21, None));
        // Never onto Java 17 for 1.16.5, though it is installed and newer.
        let old = add_instance(&app.app, "Old", "1.16.5", false, path(&l17)).await;
        assert_eq!(fix(old).await, automatic(8, None));

        // A found Java 21 fits, so it is used rather than a download.
        let l21 = add_other(
            &app.app,
            "l21",
            "local",
            "/usr/lib/jvm/java-21/bin/java",
            "21.0.8",
        )
        .await;
        assert_eq!(
            fix(on17).await,
            Some(MismatchFix {
                major: 21,
                to: MismatchFixTo::Java {
                    id: "l21".into(),
                    path: l21.path.clone(),
                    req: JavaRequirement::of_major(21),
                },
            })
        );

        // GDLauncher's Java 21: Automatic picks it.
        add_managed(&app.app, "m21", "zulu21.0.13.1-a", "21.0.13.1", 1).await;
        assert_eq!(fix(on17).await, automatic(21, None));

        // 1.17 where Java 16 downloads: the found Java 17 fits it. Then
        // GDLauncher's Java 17, which comes first, so the floor goes to 17
        // for Automatic to pick it.
        let caves = add_instance(&app.app, "Caves", "1.17", false, path(&l11)).await;
        assert_eq!(
            fix(caves).await,
            Some(MismatchFix {
                major: 17,
                to: MismatchFixTo::Java {
                    id: "l17".into(),
                    path: l17.path.clone(),
                    req: JavaRequirement::of_major(17),
                },
            })
        );
        add_managed(&app.app, "m17", "zulu17.0.13-b", "17.0.13", 1).await;
        assert_eq!(fix(caves).await, automatic(17, Some(17)));

        // Only for a Java that doesn't fit.
        let fine = add_instance(&app.app, "Fine", "1.21.4", false, None).await;
        assert_eq!(fix(fine).await, None);
    }

    /// On Windows a game runs `javaw.exe`, next to the `java.exe` its Java
    /// is listed by, and Windows paths ignore case. Pure over the paths, so
    /// it runs on every OS.
    #[test]
    fn a_windows_game_on_javaw_holds_the_java_beside_it() {
        let mut running = RunningJavas::new(JavaOs::Windows);
        running.add_path(r"C:\Users\Me\gdl\managed_javas\zulu21\bin\javaw.exe".into());

        assert!(running.uses("m21", r"c:\users\me\GDL\managed_javas\zulu21\bin\java.exe"));
        assert!(running.uses("m21", "C:/Users/Me/gdl/managed_javas/zulu21/bin/java.exe"));
        assert!(!running.uses("m17", r"C:\Users\Me\gdl\managed_javas\zulu17\bin\java.exe"));
    }

    #[test]
    fn a_java_binary_is_known_by_its_folder() {
        assert_eq!(
            java_folder_key(r"C:\Java\Zulu21\bin\javaw.exe", JavaOs::Windows),
            java_folder_key("c:/java/zulu21/bin/java.exe", JavaOs::Windows)
        );
        assert_eq!(
            java_folder_key(r"C:\Java\Zulu21\bin\java.exe", JavaOs::Windows).as_deref(),
            Some(r"c:\java\zulu21\bin")
        );
        assert_eq!(
            java_folder_key("/opt/jdk-21/bin/java", JavaOs::Linux).as_deref(),
            Some("/opt/jdk-21/bin")
        );
        assert_eq!(java_folder_key("java", JavaOs::MacOs), None);
    }

    #[test]
    fn elsewhere_a_java_is_held_by_its_own_folder_as_written() {
        let mut running = RunningJavas::new(JavaOs::Linux);
        running.add_path("/opt/jdk-21/bin/java".into());

        assert!(running.uses("j21", "/opt/jdk-21/bin/java"));
        assert!(!running.uses("j21-other", "/opt/JDK-21/bin/java"));
        assert!(!running.uses("j17", "/opt/jdk-17/bin/java"));
    }

    #[tokio::test]
    async fn details_read_the_java_catalog_only_from_cache() {
        let app = crate::setup_managers_for_test().await;
        *app.minecraft_manager().manifest_for_test.lock().unwrap() = Some(Ok(test_manifest()));
        add_managed(&app.app, "m17", "zulu17.0.13-a", "17.0.13", 1).await;
        let caves = add_instance(&app.app, "Caves", "1.17", false, None).await;

        // Nothing cached: Java 16 counts as unavailable, and nothing is fetched.
        let details = app
            .instance_manager()
            .instance_details(caves)
            .await
            .unwrap();
        assert_eq!(details.required_java.map(|req| req.major), Some(17));
        let service = &app.java_manager().managed_service;
        assert!(
            service
                .cached_catalog(crate::domain::java::JavaVendor::Azul)
                .is_none()
        );
        assert!(java_mismatches(&app.app).await.is_empty());
        assert!(
            service
                .cached_catalog(crate::domain::java::JavaVendor::Azul)
                .is_none()
        );
    }

    #[tokio::test]
    async fn instance_java_queries_follow_their_sources() {
        use crate::api::keys::{
            instance::{GET_ALL_INSTANCES, GET_JAVA_MISMATCHES, INSTANCE_DETAILS},
            java::{GET_AVAILABLE_JAVAS, GET_JAVA_ADVISORY_MATCHES, GET_JAVA_PROFILES},
        };

        let mut app = crate::setup_managers_for_test().await;
        let mut sent = |source: crate::api::keys::Key, args: Option<serde_json::Value>| {
            while app.invalidation_recv.try_recv().is_ok() {}
            app.app.invalidate(source, args);
            let mut keys = Vec::new();
            while let Ok(event) = app.invalidation_recv.try_recv() {
                keys.push(event.key);
            }
            keys
        };

        // Instance details resolve against the Javas, profiles and advisories.
        for source in [
            GET_AVAILABLE_JAVAS,
            GET_JAVA_PROFILES,
            GET_JAVA_ADVISORY_MATCHES,
        ] {
            let keys = sent(source, None);
            assert!(keys.contains(&INSTANCE_DETAILS.full), "{source}: {keys:?}");
            assert!(
                keys.contains(&GET_JAVA_MISMATCHES.full),
                "{source}: {keys:?}"
            );
        }
        // The mismatches also follow the instances.
        for source in [GET_ALL_INSTANCES, INSTANCE_DETAILS] {
            let keys = sent(source, Some(1.into()));
            assert!(
                keys.contains(&GET_JAVA_MISMATCHES.full),
                "{source}: {keys:?}"
            );
        }
    }

    #[tokio::test]
    async fn a_path_to_a_gone_gdlauncher_java_resolves_to_automatic() {
        let app = test_app(vec![]).await;
        add_managed(&app.app, "m17", "zulu17.0.13-a", "17.0.13", 1).await;
        let gone = app
            .settings_manager()
            .runtime_path
            .get_managed_javas()
            .to_path()
            .join("zulu17.0.12-old/bin/java")
            .to_string_lossy()
            .into_owned();
        let missed = add_instance(
            &app.app,
            "Missed",
            "1.20.4",
            false,
            Some(JavaOverride::Path(Some(gone))),
        )
        .await;
        // Elsewhere, a missing path is still the instance's.
        let unmounted = add_instance(
            &app.app,
            "Unmounted",
            "1.20.4",
            false,
            Some(JavaOverride::Path(Some("/media/usb/jdk/bin/java".into()))),
        )
        .await;

        let resolved = resolve_instance_java(&app.app, missed).await.unwrap();
        assert_eq!(
            (resolved.java_id.as_deref(), resolved.via),
            (Some("m17"), JavaVia::Automatic)
        );
        let details = app
            .instance_manager()
            .instance_details(missed)
            .await
            .unwrap();
        assert!(!details.java_override_mismatch);
        // The settings show what a launch does: Automatic.
        assert!(
            details.java_override.is_none(),
            "{:?}",
            details.java_override
        );
        assert_eq!(
            java_usage(&app.app).await.unwrap()["m17"],
            vec![usage_of("Missed", missed, false)]
        );
        assert_eq!(
            resolve_instance_java(&app.app, unmounted)
                .await
                .unwrap()
                .via,
            JavaVia::Instance
        );
    }

    #[tokio::test]
    async fn an_unlisted_path_resolves_without_version_data() {
        let app = test_app(vec![]).await;
        let unknown = add_instance(
            &app.app,
            "Future",
            "99.1",
            false,
            Some(JavaOverride::Path(Some("/opt/jdk/bin/java".into()))),
        )
        .await;

        let resolution = InstanceJavaResolution {
            java_id: None,
            path: Some("/opt/jdk/bin/java".into()),
            major: None,
            via: JavaVia::Instance,
            downloads_at_launch: false,
        };
        assert_eq!(
            resolve_instance_java(&app.app, unknown).await.unwrap(),
            resolution
        );

        let details = app
            .instance_manager()
            .instance_details(unknown)
            .await
            .unwrap();
        assert_eq!(details.resolved_java.unwrap().resolution, resolution);
        assert_eq!(details.required_java, None);
        assert_eq!(details.required_java_profile, None);
        assert!(!details.java_override_mismatch);

        // Nor does it make the overview's usage incomplete.
        let context = ResolveContext::load(&app.app).await.unwrap();
        let usage = usage_with(&app.app, &context, &instance_javas(&app.app).await).await;
        assert!(!usage.incomplete);
    }

    #[tokio::test]
    async fn java_mismatches_lists_instances_set_to_a_java_they_cant_use() {
        use crate::managers::java::resolve::requirement_for;

        let app = test_app(vec![]).await;
        let m17 = add_managed(&app.app, "m17", "zulu17.0.13-a", "17.0.13", 1).await;
        let m21 = add_managed(&app.app, "m21", "zulu21.0.13.1-b", "21.0.13.1", 1).await;
        let profile =
            |name: SystemJavaProfileName| Some(JavaOverride::Profile(Some(name.to_string())));

        add_instance(&app.app, "Auto", "1.21.4", false, None).await;
        add_instance(&app.app, "Fits", "1.21.4", false, path(&m21)).await;
        add_instance(
            &app.app,
            "Required",
            "1.21.4",
            false,
            profile(SystemJavaProfileName::Delta),
        )
        .await;
        add_instance(
            &app.app,
            "Unlisted",
            "1.21.4",
            false,
            path(&JavaRow {
                path: "/opt/jdk/bin/java".into(),
                ..m17.clone()
            }),
        )
        .await;
        let wrong = add_instance(&app.app, "Wrong", "1.21.4", false, path(&m17)).await;
        let old_profile = add_instance(
            &app.app,
            "Gamma",
            "1.21.4",
            false,
            profile(SystemJavaProfileName::Gamma),
        )
        .await;

        let mut mismatches = java_mismatches(&app.app).await;
        mismatches.sort_by_key(|m| m.instance_id.0);

        assert_eq!(
            mismatches,
            vec![
                JavaMismatch {
                    instance_id: wrong,
                    need: requirement_for(SystemJavaProfileName::Delta),
                    floor: None,
                },
                JavaMismatch {
                    instance_id: old_profile,
                    need: requirement_for(SystemJavaProfileName::Delta),
                    floor: None,
                },
            ]
        );
    }
}
