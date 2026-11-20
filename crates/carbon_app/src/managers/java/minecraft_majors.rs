use crate::{App, domain::java::SystemJavaProfileName};
use carbon_repos::{db_exec::Db, dbtypes::DbDateTime, repos::frontend_preference};
use daedalus::minecraft::{MinecraftJavaProfile, VersionManifest, VersionType};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use strum::IntoEnumIterator;

/// The last good [`MinecraftVersionProfiles`], as JSON.
const VERSION_PROFILES_KEY: &str = "java_minecraft_profile_snapshot";

/// How long fetched version profiles are reused before the manifest is
/// fetched again. The overview reloads often and the manifest is large.
const VERSION_PROFILES_FRESH_FOR: Duration = Duration::from_secs(10 * 60);

/// The Minecraft releases that run on one Java major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinecraftJavaMajor {
    pub major: u8,
    /// Oldest release id.
    pub from_version: String,
    /// Newest release id.
    pub to_version: String,
    /// Highest major in the list.
    pub newest: bool,
    /// Lowest major in the list.
    pub oldest: bool,
}

/// The system profile for a manifest profile name (`java-runtime-delta`) or a
/// system profile id.
fn system_profile(profile: &str) -> Option<SystemJavaProfileName> {
    let kebab = serde_json::from_value::<MinecraftJavaProfile>(serde_json::Value::String(
        profile.to_string(),
    ))
    .ok()
    .and_then(|p| SystemJavaProfileName::try_from(p).ok());

    kebab.or_else(|| SystemJavaProfileName::try_from(profile).ok())
}

fn profile_major(profile: &str) -> Option<u8> {
    system_profile(profile).map(|p| p.major())
}

/// `versions` is `(id, is_release, java_profile)` in manifest order, newest
/// first. Snapshots and unknown profiles are skipped. The result is newest
/// major first.
pub fn minecraft_java_majors(
    versions: &[(String, bool, Option<String>)],
) -> Vec<MinecraftJavaMajor> {
    let mut majors: Vec<MinecraftJavaMajor> = Vec::new();

    for (id, is_release, profile) in versions {
        if !is_release {
            continue;
        }
        let Some(major) = profile.as_deref().and_then(profile_major) else {
            continue;
        };

        match majors.iter_mut().find(|m| m.major == major) {
            Some(entry) => entry.from_version = id.clone(),
            None => majors.push(MinecraftJavaMajor {
                major,
                from_version: id.clone(),
                to_version: id.clone(),
                newest: false,
                oldest: false,
            }),
        }
    }

    majors.sort_by(|a, b| b.major.cmp(&a.major));
    if let Some(first) = majors.first_mut() {
        first.newest = true;
    }
    if let Some(last) = majors.last_mut() {
        last.oldest = true;
    }

    majors
}

/// [`minecraft_java_majors`] for the versions in Minecraft's manifest.
pub fn manifest_java_majors(manifest: &VersionManifest) -> Vec<MinecraftJavaMajor> {
    MinecraftVersionProfiles::from_manifest(manifest).majors()
}

/// One entry per Java major a system profile names, newest first, with no
/// Minecraft range: for when there is no version data at all.
pub fn profile_java_majors() -> Vec<MinecraftJavaMajor> {
    let mut majors: Vec<u8> = SystemJavaProfileName::iter()
        .filter(|profile| *profile != SystemJavaProfileName::MinecraftJavaExe)
        .map(|profile| profile.major())
        .collect();
    majors.sort_by(|a, b| b.cmp(a));
    majors.dedup();

    let last = majors.len().saturating_sub(1);
    majors
        .into_iter()
        .enumerate()
        .map(|(i, major)| MinecraftJavaMajor {
            major,
            from_version: String::new(),
            to_version: String::new(),
            newest: i == 0,
            oldest: i == last,
        })
        .collect()
}

/// What the Java pages need from Minecraft's manifest: each version and the
/// Java profile it asks for, newest first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MinecraftVersionProfiles {
    pub versions: Vec<VersionProfile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionProfile {
    pub id: String,
    pub release: bool,
    /// As the manifest names it, such as `java-runtime-delta`.
    pub profile: String,
}

impl MinecraftVersionProfiles {
    pub fn from_manifest(manifest: &VersionManifest) -> Self {
        let versions = manifest
            .versions
            .iter()
            .filter_map(|v| {
                let profile = v.java_profile.as_ref()?.as_str().ok()?;
                Some(VersionProfile {
                    id: v.id.clone(),
                    release: matches!(v.type_, VersionType::Release),
                    profile: profile.to_string(),
                })
            })
            .collect();

        Self { versions }
    }

    pub fn majors(&self) -> Vec<MinecraftJavaMajor> {
        let versions = self
            .versions
            .iter()
            .map(|v| (v.id.clone(), v.release, Some(v.profile.clone())))
            .collect::<Vec<_>>();

        minecraft_java_majors(&versions)
    }

    /// The system profile each version asks for, by version id.
    pub fn required(&self) -> HashMap<String, SystemJavaProfileName> {
        self.versions
            .iter()
            .filter_map(|v| Some((v.id.clone(), system_profile(&v.profile)?)))
            .collect()
    }
}

/// [`MinecraftVersionProfiles`] kept for the session.
pub(crate) struct CachedVersionProfiles {
    /// `None` when they come from disk rather than a fetch.
    fetched_at: Option<Instant>,
    profiles: Arc<MinecraftVersionProfiles>,
}

/// The version profiles of Minecraft's manifest. A fetch is reused for a few
/// minutes, kept for the session and stored, so when the manifest can't be
/// fetched the last good one serves instead, even from an earlier session.
/// `None` when there has never been one.
pub async fn minecraft_version_profiles(app: &App) -> Option<Arc<MinecraftVersionProfiles>> {
    let manager = app.java_manager();
    if let Some(cached) = &*manager.version_profiles.read().await
        && cached
            .fetched_at
            .is_some_and(|at| at.elapsed() < VERSION_PROFILES_FRESH_FOR)
    {
        return Some(cached.profiles.clone());
    }

    let error = match app.minecraft_manager().get_minecraft_manifest().await {
        Ok(manifest) => {
            let profiles = Arc::new(MinecraftVersionProfiles::from_manifest(&manifest));
            let changed = {
                let mut cached = manager.version_profiles.write().await;
                let changed = cached.as_ref().is_none_or(|c| c.profiles != profiles);
                *cached = Some(CachedVersionProfiles {
                    fetched_at: Some(Instant::now()),
                    profiles: profiles.clone(),
                });
                changed
            };
            if changed && let Err(e) = store_version_profiles(&app.db, &profiles).await {
                tracing::warn!("could not store the Minecraft version profiles: {e:#}");
            }

            return Some(profiles);
        }
        Err(error) => error,
    };

    tracing::warn!("could not fetch the Minecraft manifest, using the last one: {error:#}");
    let mut cached = manager.version_profiles.write().await;
    if let Some(cached) = &*cached {
        return Some(cached.profiles.clone());
    }

    let stored = stored_version_profiles(&app.db)
        .await
        .inspect_err(|e| tracing::warn!("could not read the stored version profiles: {e:#}"))
        .ok()
        .flatten()?;
    let profiles = Arc::new(stored);
    *cached = Some(CachedVersionProfiles {
        fetched_at: None,
        profiles: profiles.clone(),
    });

    Some(profiles)
}

/// The version profiles this session has, or the last stored ones, however
/// old, never fetching: for what must not wait on the network, such as the
/// check before a launch. `None` when there have never been any.
pub async fn cached_version_profiles(app: &App) -> Option<Arc<MinecraftVersionProfiles>> {
    let manager = app.java_manager();
    if let Some(cached) = &*manager.version_profiles.read().await {
        return Some(cached.profiles.clone());
    }

    let stored = stored_version_profiles(&app.db)
        .await
        .inspect_err(|e| tracing::warn!("could not read the stored version profiles: {e:#}"))
        .ok()
        .flatten()?;
    let mut cached = manager.version_profiles.write().await;
    // Fetched meanwhile: that one is newer.
    if let Some(cached) = &*cached {
        return Some(cached.profiles.clone());
    }
    let profiles = Arc::new(stored);
    *cached = Some(CachedVersionProfiles {
        fetched_at: None,
        profiles: profiles.clone(),
    });

    Some(profiles)
}

/// The Java majors Minecraft uses, with the releases each serves, from
/// [`minecraft_version_profiles`], so they work offline. With no version data
/// at all, every major a system profile names, without releases.
pub async fn current_java_majors(app: &App) -> Vec<MinecraftJavaMajor> {
    match minecraft_version_profiles(app).await {
        Some(profiles) => profiles.majors(),
        None => profile_java_majors(),
    }
}

/// For tests: makes the version profiles of this session due a fetch.
#[cfg(test)]
pub(crate) async fn age_version_profiles_for_test(app: &App) {
    if let Some(cached) = &mut *app.java_manager().version_profiles.write().await {
        cached.fetched_at = None;
    }
}

async fn store_version_profiles(
    db: &Db,
    profiles: &MinecraftVersionProfiles,
) -> anyhow::Result<()> {
    frontend_preference::upsert_preference(
        db,
        VERSION_PROFILES_KEY,
        &serde_json::to_string(profiles)?,
        DbDateTime(chrono::Utc::now().fixed_offset()),
    )
    .await?;

    Ok(())
}

/// An unparsable value reads as none.
async fn stored_version_profiles(db: &Db) -> anyhow::Result<Option<MinecraftVersionProfiles>> {
    let pref = frontend_preference::get_preference(db, VERSION_PROFILES_KEY).await?;

    Ok(pref.and_then(|p| serde_json::from_str(&p.value).ok()))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::managers::java::test_support::{manifest_unreachable, test_app};

    fn range(majors: &[MinecraftJavaMajor], major: u8) -> (String, String) {
        let m = majors.iter().find(|m| m.major == major).unwrap();
        (m.from_version.clone(), m.to_version.clone())
    }

    #[tokio::test]
    async fn java_majors_keep_their_ranges_offline() {
        let app = test_app(vec![]).await;
        let online = current_java_majors(&app.app).await;
        assert_eq!(range(&online, 21), ("1.20.5".into(), "1.21.4".into()));

        // A later session, offline: the stored snapshot serves.
        manifest_unreachable(&app.app);
        *app.java_manager().version_profiles.write().await = None;
        assert_eq!(current_java_majors(&app.app).await, online);
    }

    #[tokio::test]
    async fn java_majors_without_any_version_data_list_every_major() {
        let app = test_app(vec![]).await;
        manifest_unreachable(&app.app);

        let majors = current_java_majors(&app.app).await;
        assert_eq!(majors, profile_java_majors());
        assert!(majors.iter().any(|m| m.major == 21));
    }

    #[test]
    fn minecraft_java_majors_builds_ranges() {
        let v = |id: &str, release: bool, profile: &str| {
            (id.to_string(), release, Some(profile.to_string()))
        };
        let versions = vec![
            v("26.1", true, "java-runtime-epsilon"),
            v("1.21.11", true, "java-runtime-delta"),
            v("25w45a", false, "java-runtime-delta"),
            v("1.20.5", true, "java-runtime-delta"),
            v("1.20.4", true, "java-runtime-gamma"),
            v("1.18", true, "java-runtime-beta"),
            v("1.17", true, "java-runtime-alpha"),
            v("1.16.5", true, "jre-legacy"),
            v("1.0", true, "jre-legacy"),
        ];

        let majors = minecraft_java_majors(&versions);
        let got: Vec<_> = majors
            .iter()
            .map(|m| {
                (
                    m.major,
                    m.from_version.as_str(),
                    m.to_version.as_str(),
                    m.newest,
                    m.oldest,
                )
            })
            .collect();

        assert_eq!(
            got,
            vec![
                (25, "26.1", "26.1", true, false),
                (21, "1.20.5", "1.21.11", false, false),
                (17, "1.18", "1.20.4", false, false),
                (16, "1.17", "1.17", false, false),
                (8, "1.0", "1.16.5", false, true),
            ]
        );
    }
}
