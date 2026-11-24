//! The Java overview behind Settings › Java: every Java grouped by the
//! Minecraft versions it serves, which instances use it, and the fix for each
//! known issue. Usage comes from [`super::instance_java`], so it never
//! disagrees with what a launch does.

use super::{
    advisories::{AdvisoryMatch, RealHostCpu, match_advisories},
    build_version::JavaBuildVersion,
    instance_java::{
        JavaUsage, ResolveContext, RunningJavas, active_games, instance_javas, java_major,
        java_usage, usage_with,
    },
    java_display::vendor_display_name,
    major_defaults::get_major_defaults,
    managed::{ManagedJavaCatalog, mojang::managed_install_folder},
    minecraft_majors::{
        MinecraftJavaMajor, MinecraftVersionProfiles, minecraft_version_profiles,
        profile_java_majors,
    },
    resolve::{
        AutomaticInputs, AutomaticPick, JavaRequirement, PickReason, effective_major, host_arch,
        javas_from_rows, pick_automatic,
    },
    shown_catalogs,
    updates::get_pinned_managed_javas,
};
use crate::{
    App,
    api::keys::{
        Key,
        instance::{GET_ALL_INSTANCES, INSTANCE_DETAILS},
        java::{
            GET_AVAILABLE_JAVAS, GET_JAVA_ADVISORY_MATCHES, GET_JAVA_PROFILES,
            GET_MANAGED_JAVA_UPDATES, SYSTEM_JAVA_PROFILE_ASSIGNMENTS,
        },
    },
    domain::java::{Java, JavaArch, JavaComponentType, JavaOs, JavaVendor},
};
use anyhow::anyhow;
use carbon_repos::repos::java as java_repo;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

/// What [`served_ranges`] depends on besides the version data: the Javas
/// and the catalog. The Minecraft ranges go stale with them.
pub const RANGE_SOURCES: [Key; 2] = [GET_AVAILABLE_JAVAS, GET_MANAGED_JAVA_UPDATES];

/// The queries the overview is built from: it goes stale with each of them.
pub const OVERVIEW_SOURCES: [Key; 7] = [
    GET_AVAILABLE_JAVAS,
    GET_JAVA_PROFILES,
    SYSTEM_JAVA_PROFILE_ASSIGNMENTS,
    GET_MANAGED_JAVA_UPDATES,
    GET_JAVA_ADVISORY_MATCHES,
    GET_ALL_INSTANCES,
    INSTANCE_DETAILS,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaNativity {
    Native,
    /// x64 on Apple silicon.
    Rosetta,
    /// 32-bit x86 on a 64-bit x86 computer.
    X86OnX64,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdvisoryFix {
    /// Update this GDLauncher-installed Java to `version`.
    Update { version: String },
    /// Install that Azul Zulu build, and move this Java's users onto it.
    SwitchToAzul { build_id: String, version: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownIssue {
    pub advisory_id: String,
    pub name: String,
    pub url: String,
    pub fixed_version: Option<String>,
    pub fix: Option<AdvisoryFix>,
    /// Games GDLauncher starts on this Java run with the advisory's
    /// mitigation args, which protect them until it is updated.
    pub mitigated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaEntry {
    pub id: String,
    pub path: String,
    pub version: String,
    pub major: u8,
    pub vendor_name: String,
    pub source: JavaComponentType,
    /// Install folder size, for GDLauncher-installed Javas.
    pub size_bytes: Option<u64>,
    pub is_valid: bool,
    pub nativity: JavaNativity,
    pub pinned: bool,
    pub update_to: Option<String>,
    pub known_issues: Vec<KnownIssue>,
    pub used_by: Vec<JavaUsage>,
    /// A game that uses it is starting or running.
    pub running_game: bool,
    /// It is deleted once no game or server runs.
    pub pending_removal: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JavaGroup {
    pub major: u8,
    pub from_version: String,
    pub to_version: String,
    pub newest: bool,
    pub oldest: bool,
    /// The Java Automatic uses for this major.
    pub in_use_java_id: Option<String>,
    /// The Java in use is the user's choice.
    pub user_chosen: bool,
    /// Nothing is installed for this major, so a launch downloads it.
    pub downloads_when_needed: bool,
    /// The Java in use first, then newest first.
    pub javas: Vec<JavaEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct JavaOverview {
    /// Newest major first.
    pub groups: Vec<JavaGroup>,
    /// Javas of majors no Minecraft version uses, newest first.
    pub other_javas: Vec<JavaEntry>,
    /// GDLauncher-installed Javas nothing uses.
    pub unused_java_ids: Vec<String>,
    pub unused_size_bytes: u64,
    /// How many instances name each custom profile, by profile name. Profiles
    /// no instance names are left out.
    pub custom_profile_users: HashMap<String, u32>,
    /// Every entry's `used_by` is the whole story. False until the startup
    /// scan has loaded the instances, or while some instance's release isn't
    /// in the version data (see [`Usage::incomplete`]).
    pub usage_complete: bool,
}

/// The overview. Without the manifest it uses the last version data fetched;
/// with none ever fetched, its groups have no Minecraft ranges and nothing
/// counts as unused (see [`OverviewInputs::incomplete`]).
pub async fn get_java_overview(app: &App) -> anyhow::Result<JavaOverview> {
    let manager = app.java_manager();
    let rows = java_repo::get_all_java(&app.db).await?;
    let matches = {
        let advisories = manager.advisories.read().await;
        match_advisories(&rows, &advisories, &RealHostCpu)
    };
    // Never fetched here: offline, a fetch takes a minute. Resolving reads
    // the same cached catalog.
    let catalogs = shown_catalogs(app);
    let automatic = AutomaticInputs::new(
        javas_from_rows(rows),
        get_major_defaults(&app.db).await?,
        matches.iter().map(|m| m.java_id.clone()).collect(),
        host_arch(),
    )
    .cached_catalog_only();
    let versions = minecraft_version_profiles(app).await;
    let context = ResolveContext::new(app, automatic, versions.as_deref()).await?;
    let instances = instance_javas(app).await;
    let usage = usage_with(app, &context, &instances).await;

    let updates = manager
        .managed_updates(&catalogs)
        .await?
        .into_iter()
        .map(|update| (update.java_id, update.latest.version))
        .collect();
    let sizes = managed_sizes(app, &context.automatic.javas).await;
    let pending_removals = manager.removal_pending_ids().await;
    let majors = match &versions {
        Some(versions) => versions.majors(),
        None => profile_java_majors(),
    };

    let overview = assemble_overview(&OverviewInputs {
        majors: &majors,
        automatic: &context.automatic,
        // Only macOS matters, for Rosetta.
        host_os: JavaOs::get_current_os().unwrap_or(JavaOs::Linux),
        usage: &usage.by_java,
        incomplete: usage.incomplete,
        active_games: &active_games(app, &instances),
        pinned: &get_pinned_managed_javas(&app.db).await?,
        pending_removals: &pending_removals,
        updates: &updates,
        matches: &matches,
        azul: catalogs.get(&JavaVendor::Azul),
        sizes: &sizes,
    });

    Ok(JavaOverview {
        custom_profile_users: context.custom_profile_users(&instances),
        ..overview
    })
}

/// Ids of the Javas something uses: an instance, directly or through
/// Automatic, or a valid per-major default. Automatic's choice for a Java
/// version no instance plays is unused. `None` while what some instance uses
/// is unknown (see [`Usage::incomplete`]).
///
/// Waits on nothing, so callers can hold `removal_lock`: `versions` is read
/// beforehand, and the Java catalog only from cache. Without a cached
/// catalog, Java 17 serves Minecraft 1.17 (see
/// [`AutomaticInputs::cached_catalog_only`]), which only ever keeps more.
pub(crate) async fn javas_with_users(
    app: &App,
    versions: Option<&MinecraftVersionProfiles>,
) -> anyhow::Result<Option<HashSet<String>>> {
    let automatic = AutomaticInputs::load(app).await?.cached_catalog_only();
    let context = ResolveContext::new(app, automatic, versions).await?;
    let usage = usage_with(app, &context, &instance_javas(app).await).await;

    if usage.incomplete {
        return Ok(None);
    }

    Ok(Some(java_users(&context.automatic, &usage.by_java)))
}

/// See [`javas_with_users`].
fn java_users(
    automatic: &AutomaticInputs,
    usage: &HashMap<String, Vec<JavaUsage>>,
) -> HashSet<String> {
    let used = usage
        .iter()
        .filter(|(_, users)| !users.is_empty())
        .map(|(id, _)| id.clone());
    // A valid default is in use even for a major no group shows.
    let defaults = automatic
        .defaults
        .iter()
        .filter(|(major, id)| {
            automatic.javas.iter().any(|java| {
                &java.id == *id && java.is_valid && java_major(&java.component) == **major
            })
        })
        .map(|(_, id)| id.clone());

    used.chain(defaults).collect()
}

/// The Java Automatic uses for the Minecraft versions of `major`.
fn automatic_choice(major: u8, automatic: &AutomaticInputs) -> AutomaticPick {
    pick_automatic(
        &JavaRequirement::of_major(major),
        &automatic.javas,
        &automatic.defaults,
        automatic.host_arch,
        &automatic.flagged,
    )
}

/// Everything the overview is assembled from.
pub(crate) struct OverviewInputs<'a> {
    pub majors: &'a [MinecraftJavaMajor],
    pub automatic: &'a AutomaticInputs,
    pub host_os: JavaOs,
    pub usage: &'a HashMap<String, Vec<JavaUsage>>,
    /// Some instance's release isn't in the version data, so what it uses is
    /// unknown and nothing may count as unused.
    pub incomplete: bool,
    /// The instances whose game is starting or running, by id, with the
    /// Java its process runs on when that is known (see [`active_games`]).
    pub active_games: &'a HashMap<i32, Option<RunningJavas>>,
    pub pinned: &'a HashSet<String>,
    /// Javas already waiting to be deleted.
    pub pending_removals: &'a HashSet<String>,
    /// The build each GDLauncher Java would update to, by Java id.
    pub updates: &'a HashMap<String, JavaBuildVersion>,
    pub matches: &'a [AdvisoryMatch],
    pub azul: Option<&'a ManagedJavaCatalog>,
    /// Install folder sizes, by Java id.
    pub sizes: &'a HashMap<String, u64>,
}

pub(crate) fn assemble_overview(inputs: &OverviewInputs) -> JavaOverview {
    let automatic = inputs.automatic;
    let entries: Vec<(JavaEntry, Option<JavaBuildVersion>)> = automatic
        .javas
        .iter()
        .map(|java| {
            let version = JavaBuildVersion::parse(&java.component.version.to_string());
            (java_entry(inputs, java), version)
        })
        .collect();

    let groups: Vec<JavaGroup> = shown_ranges(inputs)
        .into_iter()
        .map(|(range, pick)| {
            let (in_use_java_id, user_chosen) = match pick {
                AutomaticPick::Java { id, reason } => (Some(id), reason == PickReason::UserDefault),
                AutomaticPick::Download { .. } => (None, false),
            };

            JavaGroup {
                major: range.major,
                from_version: range.from_version,
                to_version: range.to_version,
                newest: range.newest,
                oldest: range.oldest,
                javas: sorted_entries(&entries, |entry| entry.major == range.major),
                downloads_when_needed: in_use_java_id.is_none(),
                in_use_java_id,
                user_chosen,
            }
        })
        .collect();

    let grouped: HashSet<u8> = groups.iter().map(|group| group.major).collect();
    let other_javas = sorted_entries(&entries, |entry| !grouped.contains(&entry.major));

    let unused: Vec<&JavaEntry> = if inputs.incomplete {
        Vec::new()
    } else {
        let users = java_users(automatic, inputs.usage);
        groups
            .iter()
            .flat_map(|group| group.javas.iter())
            .chain(other_javas.iter())
            .filter(|entry| {
                entry.source == JavaComponentType::Managed
                    && !users.contains(&entry.id)
                    && !inputs.pending_removals.contains(&entry.id)
            })
            .collect()
    };

    JavaOverview {
        unused_java_ids: unused.iter().map(|entry| entry.id.clone()).collect(),
        unused_size_bytes: unused
            .iter()
            .filter_map(|entry| entry.size_bytes)
            .fold(0, u64::saturating_add),
        groups,
        other_javas,
        // Profiles aren't Javas; `get_java_overview` adds them.
        custom_profile_users: HashMap::new(),
        usage_complete: !inputs.incomplete,
    }
}

/// The ranges that get a group (see [`served_ranges`]), each with its
/// automatic choice.
fn shown_ranges(inputs: &OverviewInputs) -> Vec<(MinecraftJavaMajor, AutomaticPick)> {
    served_ranges(inputs.majors, inputs.automatic, inputs.azul)
        .into_iter()
        .map(|range| {
            let pick = automatic_choice(range.major, inputs.automatic);
            (range, pick)
        })
        .collect()
}

/// The Minecraft ranges Settings › Java groups by and the Add dialog labels
/// with, so both always agree. A major with no Java for Automatic to use and
/// no build in the Azul catalog to download (Java 16 on Linux, or any when
/// Azul was never reached) is served by another major's choice (see
/// [`effective_major`]), so its versions join that one's range. Where it
/// has either, it keeps its own.
pub(crate) fn served_ranges(
    majors: &[MinecraftJavaMajor],
    automatic: &AutomaticInputs,
    azul: Option<&ManagedJavaCatalog>,
) -> Vec<MinecraftJavaMajor> {
    let available = |major| {
        azul.is_some_and(|azul| azul.latest_for_major(major).is_some())
            || matches!(
                automatic_choice(major, automatic),
                AutomaticPick::Java { .. }
            )
    };
    let mut ranges = majors.to_vec();

    let folded: Vec<(MinecraftJavaMajor, u8)> = majors
        .iter()
        .filter_map(|range| {
            let serving = effective_major(range.major, available);
            (serving != range.major).then(|| (range.clone(), serving))
        })
        .collect();

    for (dropped, serving) in folded {
        let Some(target) = ranges.iter_mut().find(|range| range.major == serving) else {
            continue;
        };
        // The folded major is the older one: its versions come first.
        target.from_version = dropped.from_version;
        target.oldest |= dropped.oldest;
        target.newest |= dropped.newest;
        ranges.retain(|range| range.major != dropped.major);
    }

    ranges
}

/// The Minecraft ranges of the Java majors, as Settings › Java groups them
/// (see [`served_ranges`]): for the Add dialog. Reads the Java catalog only
/// from cache.
pub async fn shown_minecraft_java_majors(app: &App) -> anyhow::Result<Vec<MinecraftJavaMajor>> {
    let majors = super::minecraft_majors::current_java_majors(app).await;
    let automatic = AutomaticInputs::load(app).await?.cached_catalog_only();
    let catalogs = shown_catalogs(app);

    Ok(served_ranges(
        &majors,
        &automatic,
        catalogs.get(&JavaVendor::Azul),
    ))
}

/// The entries `keep` keeps, newest first. Only the version orders them, so
/// choosing the Java a group uses never moves a row.
fn sorted_entries(
    entries: &[(JavaEntry, Option<JavaBuildVersion>)],
    keep: impl Fn(&JavaEntry) -> bool,
) -> Vec<JavaEntry> {
    let mut kept: Vec<_> = entries.iter().filter(|(entry, _)| keep(entry)).collect();

    kept.sort_by(|(a, a_version), (b, b_version)| {
        b_version.cmp(a_version).then_with(|| a.id.cmp(&b.id))
    });

    kept.into_iter().map(|(entry, _)| entry.clone()).collect()
}

fn java_entry(inputs: &OverviewInputs, java: &Java) -> JavaEntry {
    let id = &java.id;
    let component = &java.component;
    let major = java_major(component);
    let update = inputs.updates.get(id);
    let used_by = inputs.usage.get(id).cloned().unwrap_or_default();

    JavaEntry {
        id: id.clone(),
        path: component.path.clone(),
        version: component.version.to_string(),
        major,
        vendor_name: vendor_display_name(&component.vendor, &component.path),
        source: component._type.clone(),
        size_bytes: inputs.sizes.get(id).copied(),
        is_valid: java.is_valid,
        nativity: nativity(component.arch, inputs.automatic.host_arch, inputs.host_os),
        pinned: inputs.pinned.contains(id),
        update_to: update.map(ToString::to_string),
        known_issues: inputs
            .matches
            .iter()
            .filter(|m| m.java_id == *id)
            .map(|m| KnownIssue {
                advisory_id: m.advisory.id.clone(),
                name: m.advisory.name.clone(),
                url: m.advisory.url.clone(),
                fixed_version: m.fixed.as_ref().map(ToString::to_string),
                fix: advisory_fix(m.fixed.as_ref(), update, major, inputs.azul, &used_by),
                // A match affects this Java on this computer, as the args a
                // launch adds do (see `mitigation_args`).
                mitigated: !m.advisory.mitigation_jvm_args.is_empty(),
            })
            .collect(),
        running_game: inputs
            .active_games
            .iter()
            .any(|(instance_id, launched)| match launched {
                // Its process runs on that Java, whatever its instance uses now.
                Some(launched) => launched.uses(id, &component.path),
                None => used_by
                    .iter()
                    .any(|usage| usage.instance_id.0 == *instance_id),
            }),
        pending_removal: inputs.pending_removals.contains(id),
        used_by,
    }
}

/// How a Java of `java` arch runs on this computer.
pub(crate) fn nativity(java: JavaArch, host: JavaArch, host_os: JavaOs) -> JavaNativity {
    match (java, host) {
        _ if java == host => JavaNativity::Native,
        (JavaArch::X86_64, JavaArch::Arm64) if host_os == JavaOs::MacOs => JavaNativity::Rosetta,
        (JavaArch::X86_32, JavaArch::X86_64) => JavaNativity::X86OnX64,
        _ => JavaNativity::Other,
    }
}

/// The one fix for a known issue fixed in `fixed`:
/// - none when every user of the Java needs its old build (`bounded`), since
///   no fixed build serves them;
/// - the update a GDLauncher Java has (`update`), when it is fixed;
/// - otherwise the newest Azul Zulu build of the major, when it is fixed;
/// - otherwise none.
pub(crate) fn advisory_fix(
    fixed: Option<&JavaBuildVersion>,
    update: Option<&JavaBuildVersion>,
    major: u8,
    azul: Option<&ManagedJavaCatalog>,
    users: &[JavaUsage],
) -> Option<AdvisoryFix> {
    let fixed = fixed?;
    if !users.is_empty() && users.iter().all(|user| user.bounded) {
        return None;
    }

    if let Some(update) = update.filter(|update| *update >= fixed) {
        return Some(AdvisoryFix::Update {
            version: update.to_string(),
        });
    }

    let build = azul?
        .latest_for_major(major)
        .filter(|build| build.version >= *fixed)?;

    Some(AdvisoryFix::SwitchToAzul {
        build_id: build.id.clone(),
        version: build.version.to_string(),
    })
}

/// The overview entry for one Java from its row alone: no update, fix or
/// size, and usage as far as it can be worked out. Nothing but reading the
/// row can fail, so a Java just added always gets its entry.
pub async fn java_entry_for(app: &App, java_id: &str) -> anyhow::Result<JavaEntry> {
    let manager = app.java_manager();
    let row = java_repo::get_java_by_id(&app.db, java_id)
        .await?
        .ok_or_else(|| anyhow!("Java with id {java_id} not found"))?;
    let matches = {
        let advisories = manager.advisories.read().await;
        match_advisories(std::slice::from_ref(&row), &advisories, &RealHostCpu)
    };
    let java = Java::try_from(row)?;

    let usage = java_usage(app)
        .await
        .inspect_err(|e| tracing::debug!("could not work out who uses java {java_id}: {e:#}"))
        .unwrap_or_default();
    let pinned = get_pinned_managed_javas(&app.db)
        .await
        .inspect_err(|e| tracing::debug!("could not read pinned javas: {e:#}"))
        .unwrap_or_default();
    let instances = instance_javas(app).await;
    let pending_removals = manager.removal_pending_ids().await;
    let automatic = AutomaticInputs::new(Vec::new(), HashMap::new(), HashSet::new(), host_arch());

    Ok(java_entry(
        &OverviewInputs {
            majors: &[],
            automatic: &automatic,
            host_os: JavaOs::get_current_os().unwrap_or(JavaOs::Linux),
            usage: &usage,
            incomplete: false,
            active_games: &active_games(app, &instances),
            pinned: &pinned,
            pending_removals: &pending_removals,
            updates: &HashMap::new(),
            matches: &matches,
            azul: None,
            sizes: &HashMap::new(),
        },
        &java,
    ))
}

/// Install folder sizes of the managed Javas among `javas`, by Java id. Each
/// is measured once, off the async runtime, and kept for the session.
async fn managed_sizes(app: &App, javas: &[Java]) -> HashMap<String, u64> {
    let manager = app.java_manager();
    let root = app
        .settings_manager()
        .runtime_path
        .get_managed_javas()
        .to_path();
    let managed: Vec<&Java> = javas
        .iter()
        .filter(|java| java.component._type == JavaComponentType::Managed)
        .collect();

    let missing: Vec<(String, PathBuf)> = {
        let sizes = manager.install_sizes.lock().await;
        managed
            .iter()
            .filter(|java| !sizes.contains_key(&java.id))
            .filter_map(|java| {
                let folder = managed_install_folder(&root, &java.component.path)?;
                Some((java.id.clone(), root.join(folder)))
            })
            .collect()
    };

    if !missing.is_empty() {
        let measured = tokio::task::spawn_blocking(move || {
            missing
                .into_iter()
                .filter_map(|(id, dir)| Some((id, dir_size(&dir)?)))
                .collect::<Vec<_>>()
        })
        .await;

        match measured {
            Ok(measured) => manager.install_sizes.lock().await.extend(measured),
            Err(e) => tracing::warn!("could not measure managed javas: {e}"),
        }
    }

    let sizes = manager.install_sizes.lock().await;
    managed
        .iter()
        .filter_map(|java| Some((java.id.clone(), *sizes.get(&java.id)?)))
        .collect()
}

/// The size of the files under `dir`, links not followed. `None` when it is
/// not a folder.
fn dir_size(dir: &Path) -> Option<u64> {
    if !dir.is_dir() {
        return None;
    }

    Some(
        walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter_map(|entry| entry.metadata().ok())
            .filter(|metadata| metadata.is_file())
            .map(|metadata| metadata.len())
            .fold(0, u64::saturating_add),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::keys::java::{GET_JAVA_OVERVIEW, GET_MINECRAFT_JAVA_MAJORS},
        managers::java::{
            advisories::{AffectedRange, JavaAdvisory},
            test_support::*,
        },
        setup_managers_for_test,
    };

    /// Settings › Java and the instance's Java dropdown read the overview.
    /// Offline, a catalog fetch can take a minute; they show the last
    /// catalog meanwhile.
    #[tokio::test]
    async fn the_overview_never_waits_on_a_catalog_fetch() {
        let Fixture { app, .. } = fixture().await;
        let java_manager = app.java_manager();
        let _fetching = java_manager
            .managed_service
            .hold_azul_fetch_for_test()
            .await;

        let overview = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            get_java_overview(&app.app),
        )
        .await
        .expect("the overview waited on the catalog fetch")
        .unwrap();

        assert_eq!(
            entry(&overview, "m21-old").update_to.as_deref(),
            Some("21.0.13.1")
        );
    }

    #[tokio::test]
    async fn a_stale_catalog_is_shown_and_refreshed_in_the_background() {
        use crate::managers::java::managed::CATALOG_TTL;

        let Fixture { app, .. } = fixture().await;
        let java_manager = app.java_manager();
        let service = &java_manager.managed_service;
        service
            .cache_azul_catalog_for_test(ManagedJavaCatalog::from_builds(vec![build(
                "b21",
                &[21, 0, 13, 1],
            )]))
            .await;
        service.advance_clock_for_test(CATALOG_TTL + std::time::Duration::from_secs(1));
        // Only Azul is stale.
        service
            .cache_mojang_catalog_for_test(ManagedJavaCatalog::default())
            .await;
        // The refresh then waits on this fetch, so it goes nowhere.
        let _fetching = service.hold_azul_fetch_for_test().await;

        let overview = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            get_java_overview(&app.app),
        )
        .await
        .expect("the overview waited on the refresh")
        .unwrap();

        assert_eq!(
            entry(&overview, "m21-old").update_to.as_deref(),
            Some("21.0.13.1")
        );
        assert!(
            service.claim_stale_refresh().is_none(),
            "the overview should have started a refresh"
        );
    }

    #[test]
    fn nativity_names_rosetta_and_32_bit() {
        use JavaArch::*;

        assert_eq!(
            nativity(X86_64, X86_64, JavaOs::Linux),
            JavaNativity::Native
        );
        assert_eq!(nativity(Arm64, Arm64, JavaOs::MacOs), JavaNativity::Native);
        assert_eq!(
            nativity(X86_64, Arm64, JavaOs::MacOs),
            JavaNativity::Rosetta
        );
        assert_eq!(
            nativity(X86_32, X86_64, JavaOs::Windows),
            JavaNativity::X86OnX64
        );
        assert_eq!(
            nativity(X86_64, Arm64, JavaOs::Windows),
            JavaNativity::Other
        );
        assert_eq!(nativity(Arm32, Arm64, JavaOs::Linux), JavaNativity::Other);
    }

    #[test]
    fn advisory_fix_prefers_an_update_then_azul() {
        let v = |s| JavaBuildVersion::parse(s).unwrap();
        let azul = ManagedJavaCatalog::from_builds(vec![
            build("b21-new", &[21, 0, 13, 1]),
            build("b21-old", &[21, 0, 12]),
        ]);

        // A GDLauncher Java whose update fixes it.
        assert_eq!(
            advisory_fix(
                Some(&v("21.0.13.1")),
                Some(&v("21.0.13.1")),
                21,
                Some(&azul),
                &[]
            ),
            Some(AdvisoryFix::Update {
                version: "21.0.13.1".into()
            })
        );
        // Any other Java, when Azul has a fixed build of the major.
        assert_eq!(
            advisory_fix(Some(&v("21.0.13.1")), None, 21, Some(&azul), &[]),
            Some(AdvisoryFix::SwitchToAzul {
                build_id: "b21-new".into(),
                version: "21.0.13.1".into()
            })
        );
        // An update that is still affected is no fix.
        assert_eq!(
            advisory_fix(
                Some(&v("21.0.14")),
                Some(&v("21.0.13.1")),
                21,
                Some(&azul),
                &[]
            ),
            None
        );
        // No fixed build anywhere, no catalog, or no fix released.
        assert_eq!(
            advisory_fix(Some(&v("21.0.13.1")), None, 17, Some(&azul), &[]),
            None
        );
        assert_eq!(
            advisory_fix(Some(&v("21.0.13.1")), None, 21, None, &[]),
            None
        );
        assert_eq!(advisory_fix(None, None, 21, Some(&azul), &[]), None);
    }

    #[tokio::test]
    async fn overview_groups_ranges_usage_and_unused() {
        let Fixture {
            app, auto, pathed, ..
        } = fixture().await;

        let overview = get_java_overview(&app.app).await.unwrap();

        // Azul has no Java 16 here, so Java 17 serves 1.17.
        assert_eq!(majors(&overview), vec![25, 21, 17, 8]);
        assert_groups_consistent(&overview);
        let java21 = group(&overview, 21);
        assert_eq!(
            (java21.from_version.as_str(), java21.to_version.as_str()),
            ("1.20.5", "1.21.4")
        );
        assert_eq!(java21.in_use_java_id.as_deref(), Some("m21-new"));
        assert!(!java21.user_chosen);
        assert!(!java21.downloads_when_needed);
        // Newest first.
        let ids = |group: &JavaGroup| group.javas.iter().map(|e| e.id.clone()).collect::<Vec<_>>();
        assert_eq!(ids(java21), ["m21-new", "l21", "m21-old"]);
        // Choosing a Java for the version doesn't move its row.
        crate::managers::java::major_defaults::set_major_default(
            &app.db,
            21,
            Some("m21-old".into()),
        )
        .await
        .unwrap();
        let chosen = get_java_overview(&app.app).await.unwrap();
        assert_eq!(
            group(&chosen, 21).in_use_java_id.as_deref(),
            Some("m21-old")
        );
        assert_eq!(ids(group(&chosen, 21)), ["m21-new", "l21", "m21-old"]);
        crate::managers::java::major_defaults::set_major_default(&app.db, 21, None)
            .await
            .unwrap();

        let managed = entry(&overview, "m21-new");
        assert_eq!(managed.used_by, vec![usage("Auto", auto, false)]);
        assert_eq!(managed.vendor_name, "Azul Zulu");
        assert_eq!(managed.source, JavaComponentType::Managed);
        assert_eq!(managed.size_bytes, Some(1000));
        assert_eq!(managed.nativity, JavaNativity::Native);
        assert!(!managed.running_game);

        let found = entry(&overview, "l21");
        assert_eq!(found.used_by, vec![usage("Pathed", pathed, true)]);
        assert_eq!(found.vendor_name, "OpenJDK");
        assert_eq!(found.size_bytes, None);

        assert_eq!(overview.unused_java_ids, vec!["m21-old".to_string()]);
        assert_eq!(overview.unused_size_bytes, 700);
        // The old build has a newer one to update to.
        assert_eq!(
            entry(&overview, "m21-old").update_to.as_deref(),
            Some("21.0.13.1")
        );
        assert_eq!(managed.update_to, None);

        let java25 = group(&overview, 25);
        assert_eq!(java25.in_use_java_id, None);
        assert!(java25.downloads_when_needed);
        assert!(java25.javas.is_empty());
        assert_eq!(
            (java25.from_version.as_str(), java25.newest, java25.oldest),
            ("26.1", true, false)
        );
        assert!(group(&overview, 8).oldest);

        // Java 22 serves no Minecraft version.
        assert_eq!(
            overview
                .other_javas
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            vec!["l22"]
        );

        // A game on Automatic is running: its Java can't go.
        app.instance_manager()
            .set_launch_state_for_test(
                auto,
                crate::managers::instance::run::LaunchState::Preparing(
                    crate::domain::vtask::VisualTaskId(1),
                ),
            )
            .await;
        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(entry(&overview, "m21-new").running_game);
        assert!(!entry(&overview, "l21").running_game);
    }

    /// The game started on m21-old, then an update moved its instance to
    /// m21-new while it runs: only m21-old has a game running on it.
    #[tokio::test]
    async fn a_running_game_stays_on_the_java_it_started_on() {
        use crate::managers::instance::run::{LaunchState, RunningInstance};
        use crate::managers::java::major_defaults::set_major_default;

        let Fixture { app, auto, .. } = fixture().await;
        let running = |id: &str, overview: &JavaOverview| entry(overview, id).running_game;

        // Adopted, with no Java read off its process: its instance's Java
        // stands in.
        app.instance_manager()
            .set_launch_state_for_test(
                auto,
                LaunchState::Running(RunningInstance::adopted(
                    4242,
                    1,
                    chrono::Utc::now(),
                    chrono::Utc::now(),
                )),
            )
            .await;
        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(running("m21-new", &overview));
        assert!(!running("m21-old", &overview));

        set_major_default(&app.db, 21, Some("m21-old".into()))
            .await
            .unwrap();
        let old_path = entry_path(&app.app, "m21-old").await;
        app.java_manager().note_launch_java(auto, &old_path);
        app.instance_manager()
            .set_launch_state_for_test(
                auto,
                LaunchState::Running(RunningInstance::spawned_for_test(4243)),
            )
            .await;
        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(running("m21-old", &overview));
        assert!(!running("m21-new", &overview));

        // The instance moves to m21-new.
        set_major_default(&app.db, 21, None).await.unwrap();
        let overview = get_java_overview(&app.app).await.unwrap();
        assert_eq!(
            entry(&overview, "m21-new").used_by,
            vec![usage("Auto", auto, false)]
        );
        assert!(running("m21-old", &overview));
        assert!(!running("m21-new", &overview));

        // So does the entry a single Java gets.
        assert!(
            java_entry_for(&app.app, "m21-old")
                .await
                .unwrap()
                .running_game
        );
        assert!(
            !java_entry_for(&app.app, "m21-new")
                .await
                .unwrap()
                .running_game
        );
    }

    /// Unused means no instance uses it and it isn't a Java version's
    /// default: Automatic's pick for a Java version nothing plays is unused.
    #[tokio::test]
    async fn automatics_pick_that_no_instance_uses_is_unused() {
        let Fixture { app, .. } = fixture().await;
        add_managed(&app.app, "m17", "zulu17.0.13-c", "17.0.13", 300).await;
        add_managed(&app.app, "m8", "zulu8.0.492-d", "8.0.492", 200).await;
        crate::managers::java::major_defaults::set_major_default(&app.db, 8, Some("m8".into()))
            .await
            .unwrap();

        let overview = get_java_overview(&app.app).await.unwrap();
        assert_eq!(group(&overview, 17).in_use_java_id.as_deref(), Some("m17"));
        let mut unused = overview.unused_java_ids.clone();
        unused.sort();
        assert_eq!(unused, ["m17", "m21-old"]);
        assert_eq!(overview.unused_size_bytes, 1000);

        let versions = minecraft_version_profiles(&app.app).await;
        let users = javas_with_users(&app.app, versions.as_deref())
            .await
            .unwrap()
            .unwrap();
        assert!(
            users.contains("m8") && users.contains("m21-new"),
            "{users:?}"
        );
        assert!(!users.contains("m17"), "{users:?}");
    }

    /// Until the startup scan has loaded the instances, what they use is
    /// unknown, so nothing counts as unused.
    #[tokio::test]
    async fn nothing_is_unused_before_the_instance_scan() {
        let Fixture { app, .. } = fixture().await;
        app.instance_manager().instances_scanned.send_replace(false);

        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(overview.unused_java_ids.is_empty(), "{overview:#?}");
        let versions = minecraft_version_profiles(&app.app).await;
        assert_eq!(
            javas_with_users(&app.app, versions.as_deref())
                .await
                .unwrap(),
            None
        );
        let removed = super::super::overview_actions::remove_unused_javas(&app.app)
            .await
            .unwrap();
        assert_eq!(removed.count, 0);
        assert!(
            java_repo::get_java_by_id(&app.db, "m21-old")
                .await
                .unwrap()
                .is_some()
        );

        app.instance_manager().instances_scanned.send_replace(true);
        let overview = get_java_overview(&app.app).await.unwrap();
        assert_eq!(overview.unused_java_ids, vec!["m21-old".to_string()]);
    }

    /// The UI mutes a found Java's known issue when nothing uses it, so it
    /// must know when "nothing" is only the scan not being done yet.
    #[tokio::test]
    async fn the_overview_says_whether_usage_is_complete() {
        let Fixture { app, .. } = fixture().await;
        app.instance_manager().instances_scanned.send_replace(false);
        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(!overview.usage_complete);

        app.instance_manager().instances_scanned.send_replace(true);
        let overview = get_java_overview(&app.app).await.unwrap();
        assert!(overview.usage_complete);
    }

    #[tokio::test]
    async fn overview_counts_the_instances_naming_each_custom_profile() {
        use crate::domain::{instance::info::JavaOverride, java::SystemJavaProfileName};

        let Fixture { app, .. } = fixture().await;
        let java = app.java_manager();
        java.create_java_profile("Mine".into(), Some("m21-new".into()))
            .await
            .unwrap();
        java.create_java_profile("Spare".into(), Some("l21".into()))
            .await
            .unwrap();
        let named = |name: &str| Some(JavaOverride::Profile(Some(name.into())));
        add_instance(&app.app, "A", "1.21.4", false, named("Mine")).await;
        add_instance(&app.app, "B", "1.20.1", false, named("Mine")).await;
        // Neither a system profile nor one that no longer exists counts.
        add_instance(
            &app.app,
            "C",
            "1.21.4",
            false,
            named(&SystemJavaProfileName::Delta.to_string()),
        )
        .await;
        add_instance(&app.app, "D", "1.21.4", false, named("Deleted")).await;

        let overview = get_java_overview(&app.app).await.unwrap();

        assert_eq!(
            overview.custom_profile_users,
            HashMap::from([("Mine".to_string(), 2)])
        );
    }

    #[tokio::test]
    async fn deleting_a_custom_profiles_java_moves_its_instances_to_automatic() {
        use crate::domain::instance::info::JavaOverride;

        let Fixture { app, auto, .. } = fixture().await;
        app.java_manager()
            .create_java_profile("Old".into(), Some("m21-old".into()))
            .await
            .unwrap();
        let profiled = add_instance(
            &app.app,
            "Profiled",
            "1.21.4",
            false,
            Some(JavaOverride::Profile(Some("Old".into()))),
        )
        .await;
        let overview = get_java_overview(&app.app).await.unwrap();
        assert_eq!(
            entry(&overview, "m21-old").used_by,
            vec![JavaUsage {
                via_profile: Some("Old".into()),
                ..usage("Profiled", profiled, true)
            }]
        );

        app.java_manager()
            .delete_java_version("m21-old".into())
            .await
            .unwrap();

        // The profile is empty now, and its instance uses Automatic's pick.
        let overview = get_java_overview(&app.app).await.unwrap();
        assert_eq!(
            entry(&overview, "m21-new").used_by,
            vec![
                usage("Auto", auto, false),
                usage("Profiled", profiled, false)
            ]
        );
        assert_eq!(
            overview.custom_profile_users,
            HashMap::from([("Old".to_string(), 1)])
        );
    }

    /// A launch adds an advisory's mitigation args to every Java it affects
    /// here, so games GDLauncher starts are protected from it.
    #[tokio::test]
    async fn known_issues_say_whether_a_launch_mitigates_them() {
        let Fixture { app, .. } = fixture().await;
        let advisory = |id: &str, args: &[&str]| JavaAdvisory {
            id: id.into(),
            name: id.into(),
            severity: "critical".into(),
            url: String::new(),
            affected: vec![AffectedRange {
                major: 21,
                introduced: None,
                fixed: None,
            }],
            arch: vec![],
            cpu_features: vec![],
            mitigation_jvm_args: args.iter().map(|arg| arg.to_string()).collect(),
        };
        *app.java_manager().advisories.write().await = vec![
            advisory("CVE-1", &["-XX:-UseBASE64Intrinsics"]),
            advisory("CVE-2", &[]),
        ];

        let overview = get_java_overview(&app.app).await.unwrap();

        let mitigated: Vec<(String, bool)> = entry(&overview, "m21-new")
            .known_issues
            .iter()
            .map(|issue| (issue.advisory_id.clone(), issue.mitigated))
            .collect();
        assert_eq!(
            mitigated,
            [("CVE-1".to_string(), true), ("CVE-2".to_string(), false)]
        );
    }

    #[tokio::test]
    async fn overview_shows_known_issues_with_their_fix() {
        let Fixture { app, .. } = fixture().await;
        *app.java_manager().advisories.write().await = vec![JavaAdvisory {
            id: "CVE-1".into(),
            name: "Java4Shell".into(),
            severity: "critical".into(),
            url: "https://example.com/cve-1".into(),
            affected: vec![AffectedRange {
                major: 21,
                introduced: None,
                fixed: Some("21.0.13.1".into()),
            }],
            arch: vec![],
            cpu_features: vec![],
            mitigation_jvm_args: vec![],
        }];

        let overview = get_java_overview(&app.app).await.unwrap();

        let issue = |id: &str| entry(&overview, id).known_issues.clone();
        assert!(issue("m21-new").is_empty());
        assert_eq!(
            issue("m21-old"),
            vec![KnownIssue {
                advisory_id: "CVE-1".into(),
                name: "Java4Shell".into(),
                url: "https://example.com/cve-1".into(),
                fixed_version: Some("21.0.13.1".into()),
                fix: Some(AdvisoryFix::Update {
                    version: "21.0.13.1".into()
                }),
                mitigated: false,
            }]
        );
        assert_eq!(
            issue("l21")[0].fix,
            Some(AdvisoryFix::SwitchToAzul {
                build_id: "b21".into(),
                version: "21.0.13.1".into()
            })
        );
    }

    #[tokio::test]
    async fn overview_follows_its_sources() {
        let mut app = setup_managers_for_test().await;
        while app.invalidation_recv.try_recv().is_ok() {}

        for source in [
            GET_ALL_INSTANCES,
            GET_JAVA_ADVISORY_MATCHES,
            GET_MANAGED_JAVA_UPDATES,
        ] {
            app.app.invalidate(source, None);

            let mut keys = Vec::new();
            while let Ok(event) = app.invalidation_recv.try_recv() {
                keys.push(event.key);
            }
            assert!(keys.contains(&GET_JAVA_OVERVIEW.full), "{source}: {keys:?}");
        }
    }

    #[test]
    fn advisory_fix_offers_nothing_when_every_user_needs_the_old_build() {
        let v = |s| JavaBuildVersion::parse(s).unwrap();
        let azul = ManagedJavaCatalog::from_builds(vec![build("b8", &[8, 0, 492])]);
        let user = |name: &str, bounded| JavaUsage {
            bounded,
            ..usage(name, InstanceId(1), true)
        };
        let fixed = v("8.0.400");

        // Forge 1.16.5 needs Java 8 below update 312: nothing fixed serves it.
        assert_eq!(
            advisory_fix(Some(&fixed), None, 8, Some(&azul), &[user("Forge", true)]),
            None
        );
        assert_eq!(
            advisory_fix(
                Some(&fixed),
                Some(&v("8.0.492")),
                8,
                Some(&azul),
                &[user("Forge", true)]
            ),
            None
        );
        // One other user is enough, and so is none.
        let switch = Some(AdvisoryFix::SwitchToAzul {
            build_id: "b8".into(),
            version: "8.0.492".into(),
        });
        assert_eq!(
            advisory_fix(
                Some(&fixed),
                None,
                8,
                Some(&azul),
                &[user("Forge", true), user("Vanilla", false)]
            ),
            switch
        );
        assert_eq!(
            advisory_fix(Some(&fixed), None, 8, Some(&azul), &[]),
            switch
        );
    }

    #[tokio::test]
    async fn java_16_folds_into_17_where_no_java_16_downloads() {
        // Linux: Azul has no Java 16.
        let app = test_app(vec![build("b17", &[17, 0, 13])]).await;
        add_other(
            &app.app,
            "l16",
            "local",
            "/usr/lib/jvm/java-16/bin/java",
            "16.0.2",
        )
        .await;

        let overview = get_java_overview(&app.app).await.unwrap();

        assert_eq!(majors(&overview), vec![25, 21, 17, 8]);
        let java17 = group(&overview, 17);
        assert_eq!(
            (java17.from_version.as_str(), java17.to_version.as_str()),
            ("1.17", "1.20.4")
        );
        // A found Java 16 has no group of its own.
        assert_eq!(
            overview
                .other_javas
                .iter()
                .map(|e| e.id.as_str())
                .collect::<Vec<_>>(),
            vec!["l16"]
        );
        assert_groups_consistent(&overview);
        assert_ranges_as_shown(&app.app, &overview).await;
    }

    /// The Add dialog's Minecraft ranges are the overview's.
    async fn assert_ranges_as_shown(app: &App, overview: &JavaOverview) {
        let range = |m: &MinecraftJavaMajor| {
            (
                m.major,
                m.from_version.clone(),
                m.to_version.clone(),
                m.newest,
                m.oldest,
            )
        };
        let shown: Vec<_> = shown_minecraft_java_majors(app)
            .await
            .unwrap()
            .iter()
            .map(range)
            .collect();
        let groups: Vec<_> = overview
            .groups
            .iter()
            .map(|g| {
                (
                    g.major,
                    g.from_version.clone(),
                    g.to_version.clone(),
                    g.newest,
                    g.oldest,
                )
            })
            .collect();

        assert_eq!(shown, groups);
    }

    #[tokio::test]
    async fn minecraft_ranges_follow_the_javas_and_the_catalog() {
        let mut app = setup_managers_for_test().await;
        while app.invalidation_recv.try_recv().is_ok() {}

        for source in [GET_AVAILABLE_JAVAS, GET_MANAGED_JAVA_UPDATES] {
            app.app.invalidate(source, None);

            let mut keys = Vec::new();
            while let Ok(event) = app.invalidation_recv.try_recv() {
                keys.push(event.key);
            }
            assert!(
                keys.contains(&GET_MINECRAFT_JAVA_MAJORS.full),
                "{source}: {keys:?}"
            );
        }
    }

    #[tokio::test]
    async fn java_16_keeps_its_group_where_it_downloads_or_is_installed() {
        // Azul has a Java 16 to download.
        let app = test_app(vec![build("b17", &[17, 0, 13]), build("b16", &[16, 0, 2])]).await;
        let overview = get_java_overview(&app.app).await.unwrap();
        assert_eq!(majors(&overview), vec![25, 21, 17, 16, 8]);
        assert!(group(&overview, 16).downloads_when_needed);
        assert_eq!(group(&overview, 17).from_version, "1.18");
        assert_groups_consistent(&overview);
        assert_ranges_as_shown(&app.app, &overview).await;

        // No Java 16 to download, but one installed by GDLauncher.
        let app = test_app(vec![]).await;
        add_managed(&app.app, "m16", "zulu16.0.2-a", "16.0.2", 1).await;
        let overview = get_java_overview(&app.app).await.unwrap();
        assert_eq!(majors(&overview), vec![25, 21, 17, 16, 8]);
        assert_eq!(group(&overview, 16).in_use_java_id.as_deref(), Some("m16"));
        assert_groups_consistent(&overview);
        assert_ranges_as_shown(&app.app, &overview).await;

        // Or the user's choice of a found one.
        let app = test_app(vec![]).await;
        add_other(
            &app.app,
            "l16",
            "local",
            "/usr/lib/jvm/java-16/bin/java",
            "16.0.2",
        )
        .await;
        crate::managers::java::major_defaults::set_major_default(&app.db, 16, Some("l16".into()))
            .await
            .unwrap();
        let overview = get_java_overview(&app.app).await.unwrap();
        assert_eq!(group(&overview, 16).in_use_java_id.as_deref(), Some("l16"));
        assert!(overview.other_javas.is_empty());
    }

    #[tokio::test]
    async fn overview_falls_back_to_the_last_version_data() {
        let Fixture { mut app, auto, .. } = fixture().await;
        // A good fetch keeps the version data, in memory and on disk.
        get_java_overview(&app.app).await.unwrap();

        app.restart_in_place().await;
        manifest_unreachable(&app.app);
        cache_test_catalogs(&app.app, vec![build("b21", &[21, 0, 13, 1])]).await;

        let overview = get_java_overview(&app.app).await.unwrap();

        assert_eq!(majors(&overview), vec![25, 21, 17, 8]);
        let java21 = group(&overview, 21);
        assert_eq!(
            (java21.from_version.as_str(), java21.to_version.as_str()),
            ("1.20.5", "1.21.4")
        );
        assert_eq!(
            entry(&overview, "m21-new").used_by,
            vec![usage("Auto", auto, false)]
        );
        assert_eq!(overview.unused_java_ids, vec!["m21-old".to_string()]);
    }

    #[tokio::test]
    async fn overview_without_any_version_data_keeps_every_java() {
        let Fixture { app, pathed, .. } = fixture().await;
        manifest_unreachable(&app.app);

        let overview = get_java_overview(&app.app).await.unwrap();

        // One group per Java version, without Minecraft ranges.
        assert_eq!(majors(&overview), vec![25, 21, 17, 8]);
        for group in &overview.groups {
            assert_eq!(
                (group.from_version.as_str(), group.to_version.as_str()),
                ("", "")
            );
        }
        assert!(group(&overview, 25).newest);
        assert!(group(&overview, 8).oldest);
        assert_groups_consistent(&overview);
        assert_eq!(
            group(&overview, 21).in_use_java_id.as_deref(),
            Some("m21-new")
        );
        // A Java set in an instance still shows who uses it.
        assert_eq!(
            entry(&overview, "l21").used_by,
            vec![usage("Pathed", pathed, true)]
        );
        // What Automatic gives the other instance is unknown, so nothing
        // counts as unused.
        assert!(entry(&overview, "m21-new").used_by.is_empty());
        assert!(overview.unused_java_ids.is_empty());
        assert_eq!(overview.unused_size_bytes, 0);
    }
}
