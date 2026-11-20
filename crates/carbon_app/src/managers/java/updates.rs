use super::{
    JavaManager,
    build_version::JavaBuildVersion,
    instance_java::{InstanceJava, RunningJavas, instance_javas, running_javas},
    invalidate_java_lists,
    major_defaults::move_major_defaults,
    managed::{
        ManagedJavaBuild, ManagedJavaCatalog,
        mojang::{install_folder_for, managed_install_folder, managed_vendor_for_path},
    },
    minecraft_majors::minecraft_version_profiles,
    overview::javas_with_users,
    resolve::bounded_requirement,
};
use crate::api::keys::java::GET_MANAGED_JAVA_UPDATES;
use crate::domain::instance::{
    InstanceId,
    info::{GameVersion, JavaOverride},
};
use crate::domain::java::{JavaVendor, JavaVersion, SystemJavaProfileName};
use crate::managers::ManagerRef;
use carbon_repos::db_exec::Db;
use carbon_repos::dbtypes::DbDateTime;
use carbon_repos::repos::frontend_preference;
use carbon_repos::repos::java::{self as java_repo, JavaProfileRow, JavaRow};
use std::collections::{HashMap, HashSet};
use std::path::Path;

const MANAGED_JAVA_PINNED_KEY: &str = "managed_java_pinned";

/// Ids of managed Javas that auto-update must leave alone. A missing or
/// unparsable value reads as empty.
pub(crate) async fn get_pinned_managed_javas(db: &Db) -> anyhow::Result<HashSet<String>> {
    let pref = frontend_preference::get_preference(db, MANAGED_JAVA_PINNED_KEY).await?;

    Ok(pref
        .and_then(|p| serde_json::from_str::<HashSet<String>>(&p.value).ok())
        .unwrap_or_default())
}

pub(crate) async fn set_pinned_managed_javas(db: &Db, ids: &HashSet<String>) -> anyhow::Result<()> {
    let mut sorted: Vec<&String> = ids.iter().collect();
    sorted.sort();
    let value = serde_json::to_string(&sorted)?;

    frontend_preference::upsert_preference(
        db,
        MANAGED_JAVA_PINNED_KEY,
        &value,
        DbDateTime(chrono::Utc::now().fixed_offset()),
    )
    .await?;

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplaceOutcome {
    /// The old Java is deleted.
    Removed,
    /// A game is starting or running, so the old Java is deleted once it is not.
    Deferred,
    /// A profile that cannot use the new Java still points at the old one, so
    /// it stays, pinned.
    Kept,
}

/// Reads the pinned set, or `None` (after a warning) when the read fails, so
/// callers never write back a set built from nothing.
async fn read_pinned_or_warn(db: &Db) -> Option<HashSet<String>> {
    match get_pinned_managed_javas(db).await {
        Ok(pinned) => Some(pinned),
        Err(e) => {
            tracing::warn!("could not read pinned managed javas: {e:#}");
            None
        }
    }
}

impl ManagerRef<'_, JavaManager> {
    /// Moves the profiles, instance path overrides and per-major defaults that
    /// use `old` onto `new`, a build of the same major. What `new` can't serve
    /// stays on `old`:
    /// - a system profile `new` doesn't satisfy;
    /// - an instance that names `old`, by path or through a custom profile,
    ///   and needs a build below `new` (Forge 1.16.5, see
    ///   [`bounded_requirement`]).
    ///
    /// Returns whether anything stayed.
    pub async fn move_java_users(&self, old: &JavaRow, new: &JavaRow) -> anyhow::Result<bool> {
        let new_version = JavaVersion::try_from(&*new.full_version)?;
        let instances = instance_javas(self.app).await;
        let breaks = |instance: &InstanceJava| move_breaks(instance.version.as_ref(), old, new);
        let mut blocked = false;

        for profile in java_repo::get_all_profiles(&self.app.db).await? {
            if profile.java_id.as_deref() != Some(old.id.as_str()) {
                continue;
            }

            let compatible = if profile.is_system_profile {
                SystemJavaProfileName::try_from(profile.name.as_str())
                    .map(|name| name.is_java_version_compatible(&new_version))
                    .unwrap_or(true)
            } else {
                !instances.iter().any(|instance| {
                    matches!(
                        &instance.java_override,
                        Some(JavaOverride::Profile(Some(name))) if *name == profile.name
                    ) && breaks(instance)
                })
            };

            if compatible {
                java_repo::set_profile_java(&self.app.db, &profile.name, Some(&new.id)).await?;
            } else {
                blocked = true;
            }
        }

        let stay: HashSet<InstanceId> = instances
            .iter()
            .filter(|instance| {
                matches!(
                    &instance.java_override,
                    Some(JavaOverride::Path(Some(path))) if *path == old.path
                ) && breaks(instance)
            })
            .map(|instance| instance.id)
            .collect();
        blocked |= !stay.is_empty();
        self.app
            .instance_manager()
            .repoint_java_path_overrides_except(&old.path, &new.path, &stay)
            .await?;
        // The user chose the Java for its major, not that build.
        move_major_defaults(&self.app.db, &old.id, &new.id).await?;

        Ok(blocked)
    }

    /// Moves what uses `old` onto `new`, then deletes `old`, defers that while
    /// a game starts or runs, or keeps it when something can't use `new`.
    pub async fn replace_managed_java(
        &self,
        old: &JavaRow,
        new: &JavaRow,
    ) -> anyhow::Result<ReplaceOutcome> {
        let blocked = self.move_java_users(old, new).await?;

        let outcome = if blocked {
            if let Some(mut pinned) = read_pinned_or_warn(&self.app.db).await {
                pinned.insert(old.id.clone());
                set_pinned_managed_javas(&self.app.db, &pinned).await?;
            }
            ReplaceOutcome::Kept
        } else {
            if let Some(mut pinned) = read_pinned_or_warn(&self.app.db).await
                && pinned.remove(&old.id)
            {
                set_pinned_managed_javas(&self.app.db, &pinned).await?;
            }

            if self.java_in_use_by_running().await {
                self.pending_removals.lock().await.insert(old.id.clone());
                ReplaceOutcome::Deferred
            } else {
                self.delete_java_version(old.id.clone()).await?;
                ReplaceOutcome::Removed
            }
        };

        invalidate_java_lists(self.app);

        Ok(outcome)
    }

    /// Deletes the Javas whose removal was deferred. The ones the user deleted
    /// go first, whatever uses them, each once no running game or server may
    /// run on it (see [`running_javas`]). The others wait while any game or
    /// server runs: a replaced Java may still run a game started before. Of
    /// those, a Java that gained a user while it waited (see
    /// [`javas_with_users`]) is kept and no longer pending; a replaced Java's
    /// users moved already, so it goes. While what some instance uses is
    /// unknown, none of those go yet. A Java that cannot be deleted stays
    /// pending and is logged, and the others still go. Returns how many were
    /// deleted.
    pub async fn process_pending_removals(&self) -> anyhow::Result<usize> {
        // Read before taking removal_lock, since it may wait on the network.
        // The lists are free meanwhile, so a replace can still add to them.
        let nothing_pending = self.pending_removals.lock().await.is_empty();
        let nothing_requested = self.requested_removals.lock().await.is_empty();
        let versions = match nothing_pending && nothing_requested {
            true => None,
            false => minecraft_version_profiles(self.app).await,
        };
        let _guard = self.removal_lock.lock().await;

        let requested = match nothing_requested {
            true => 0,
            false => {
                let running = running_javas(self.app, versions.as_deref()).await?;
                self.delete_requested_removals(&running).await?
            }
        };
        if self.java_in_use_by_running().await {
            return Ok(requested);
        }

        let ids: Vec<String> = self.pending_removals.lock().await.iter().cloned().collect();
        if ids.is_empty() {
            return Ok(requested);
        }
        let Some(users) = javas_with_users(self.app, versions.as_deref()).await? else {
            tracing::info!("not deleting deferred javas yet: what some instance uses is unknown");
            return Ok(requested);
        };
        let mut removed = requested;

        for id in ids {
            if users.contains(&id) {
                tracing::info!("keeping java {id}: something uses it again");
                self.pending_removals.lock().await.remove(&id);
                continue;
            }

            if java_repo::get_java_by_id(&self.app.db, &id)
                .await?
                .is_some()
            {
                if let Err(e) = self.delete_java_version(id.clone()).await {
                    tracing::warn!("could not delete replaced java {id}: {e:#}");
                    continue;
                }
                removed += 1;
            }
            self.pending_removals.lock().await.remove(&id);
        }

        invalidate_java_lists(self.app);

        Ok(removed)
    }

    /// Deletes the Javas the user deleted while a game ran on them, except
    /// the ones a `running` game or server still may run on. Callers hold
    /// `removal_lock`.
    async fn delete_requested_removals(&self, running: &RunningJavas) -> anyhow::Result<usize> {
        let ids: Vec<String> = self
            .requested_removals
            .lock()
            .await
            .iter()
            .cloned()
            .collect();
        if ids.is_empty() {
            return Ok(0);
        }

        let mut removed = 0;
        for id in ids {
            if let Some(java) = java_repo::get_java_by_id(&self.app.db, &id).await? {
                if running.uses(&java.id, &java.path) {
                    continue;
                }
                if let Err(e) = self.delete_java_version(id.clone()).await {
                    tracing::warn!("could not delete java {id} the user deleted: {e:#}");
                    continue;
                }
                removed += 1;
            }
            self.requested_removals.lock().await.remove(&id);
        }
        invalidate_java_lists(self.app);

        Ok(removed)
    }
}

/// Whether moving an instance of `version` from `old` to `new`, builds of one
/// major, breaks it: `new` misses a cap on the build that `old` meets.
fn move_breaks(version: Option<&GameVersion>, old: &JavaRow, new: &JavaRow) -> bool {
    let Some(GameVersion::Standard(version)) = version else {
        return false;
    };
    let Some(req) = bounded_requirement(version) else {
        return false;
    };
    let meets = |row: &JavaRow| {
        let major = u8::try_from(row.major).ok();
        let version = JavaBuildVersion::parse(&row.full_version);
        matches!((major, version), (Some(major), Some(version)) if req.accepts(major, &version))
    };

    meets(old) && !meets(new)
}

/// A managed Java that has a newer build available in the catalog.
#[derive(Debug, Clone)]
pub struct ManagedJavaUpdate {
    pub java_id: String,
    /// The distribution the Java came from, and the one `latest` is from.
    pub vendor: JavaVendor,
    pub current: JavaBuildVersion,
    pub latest: ManagedJavaBuild,
}

/// Valid managed Javas whose major has a strictly newer build in their own
/// distribution's catalog. A Java whose distribution is unknown, or has no
/// catalog, is skipped.
pub fn find_managed_updates(
    installed: &[JavaRow],
    catalogs: &HashMap<JavaVendor, ManagedJavaCatalog>,
    managed_root: &Path,
) -> Vec<ManagedJavaUpdate> {
    installed
        .iter()
        .filter(|row| row.r#type == "managed" && row.is_valid)
        .filter_map(|row| {
            let vendor = managed_vendor_for_path(managed_root, &row.path)?;
            let catalog = catalogs.get(&vendor)?;
            let current = JavaBuildVersion::parse(&row.full_version)?;
            let major = u8::try_from(row.major).ok()?;
            let latest = catalog.latest_for_major(major)?;

            // A build that installs into the Java's own folder is the Java
            // itself, whatever version its binary reports.
            if install_folder_for(latest).is_some()
                && install_folder_for(latest) == managed_install_folder(managed_root, &row.path)
            {
                return None;
            }

            (latest.version > current).then(|| ManagedJavaUpdate {
                java_id: row.id.clone(),
                vendor,
                current,
                latest: latest.clone(),
            })
        })
        .collect()
}

/// The updates auto-update should run: none when it is off, and never a
/// pinned Java.
pub fn updates_to_apply<'a>(
    updates: &'a [ManagedJavaUpdate],
    auto_update: bool,
    pinned: &HashSet<String>,
) -> Vec<&'a ManagedJavaUpdate> {
    if !auto_update {
        return Vec::new();
    }

    updates
        .iter()
        .filter(|update| !pinned.contains(&update.java_id))
        .collect()
}

/// Drops updates that would be offered again right after they ran:
/// - Javas waiting for a deferred delete;
/// - Javas a system profile uses that the newest build of the major would not
///   satisfy, because replacing keeps the old Java for that profile.
fn without_looping_updates(
    updates: Vec<ManagedJavaUpdate>,
    profiles: &[JavaProfileRow],
    pending_removals: &HashSet<String>,
) -> Vec<ManagedJavaUpdate> {
    updates
        .into_iter()
        .filter(|update| !pending_removals.contains(&update.java_id))
        .filter(|update| {
            let latest = update.latest.java_version();

            !profiles.iter().any(|profile| {
                profile.is_system_profile
                    && profile.java_id.as_deref() == Some(update.java_id.as_str())
                    && SystemJavaProfileName::try_from(profile.name.as_str())
                        .is_ok_and(|name| !name.is_java_version_compatible(&latest))
            })
        })
        .collect()
}

impl ManagerRef<'_, JavaManager> {
    /// Updates `find_managed_updates` reports, minus the ones that would loop.
    pub async fn managed_updates(
        &self,
        catalogs: &HashMap<JavaVendor, ManagedJavaCatalog>,
    ) -> anyhow::Result<Vec<ManagedJavaUpdate>> {
        let javas = java_repo::get_all_java(&self.app.db).await?;
        let profiles = java_repo::get_all_profiles(&self.app.db).await?;
        let pending = self.removal_pending_ids().await;

        let managed_root = self
            .app
            .settings_manager()
            .runtime_path
            .get_managed_javas()
            .to_path();

        Ok(without_looping_updates(
            find_managed_updates(&javas, catalogs, &managed_root),
            &profiles,
            &pending,
        ))
    }

    /// Ids among `fixes` (java id, version that fixes it) that auto-update
    /// will really fix: the updater would replace them and the newest build of
    /// their major is at least the fixed version. Empty without catalogs.
    pub async fn auto_fixable_ids(
        &self,
        catalogs: &HashMap<JavaVendor, ManagedJavaCatalog>,
        fixes: &[(String, Option<JavaBuildVersion>)],
    ) -> anyhow::Result<HashSet<String>> {
        if catalogs.is_empty() {
            return Ok(HashSet::new());
        }

        let auto_update = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .auto_update_managed_java;
        let pinned = get_pinned_managed_javas(&self.app.db).await?;
        let updates = self.managed_updates(catalogs).await?;

        Ok(updates_to_apply(&updates, auto_update, &pinned)
            .into_iter()
            .filter(|update| {
                fixes.iter().any(|(id, fixed)| {
                    *id == update.java_id
                        && fixed.as_ref().is_some_and(|f| update.latest.version >= *f)
                })
            })
            .map(|update| update.java_id.clone())
            .collect())
    }

    /// Replaces a managed Java with the newest build of its major and returns
    /// the new Java's id. Errors when the Java is not managed or is already
    /// the newest.
    pub async fn update_managed_java(&self, java_id: &str) -> anyhow::Result<String> {
        let old = java_repo::get_java_by_id(&self.app.db, java_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Java with id {java_id} not found"))?;

        if old.r#type != "managed" {
            anyhow::bail!("Java with id {java_id} is not managed");
        }

        let managed_root = self
            .app
            .settings_manager()
            .runtime_path
            .get_managed_javas()
            .to_path();
        let vendor = managed_vendor_for_path(&managed_root, &old.path)
            .ok_or_else(|| anyhow::anyhow!("Cannot tell where Java {java_id} came from"))?;
        let catalog = self.managed_service.get_catalog(vendor).await?;
        let major = u8::try_from(old.major)?;
        let latest = catalog
            .latest_for_major(major)
            .ok_or_else(|| anyhow::anyhow!("No build available for Java {major}"))?;
        let current = JavaBuildVersion::parse(&old.full_version)
            .ok_or_else(|| anyhow::anyhow!("Unrecognised version {}", old.full_version))?;

        if latest.version <= current
            || (install_folder_for(latest).is_some()
                && install_folder_for(latest) == managed_install_folder(&managed_root, &old.path))
        {
            anyhow::bail!("Java {} is already up to date", old.full_version);
        }

        let installed = java_repo::get_all_java(&self.app.db)
            .await?
            .into_iter()
            .find(|row| {
                row.id != old.id
                    && managed_vendor_for_path(&managed_root, &row.path) == Some(vendor)
                    && row.r#type == "managed"
                    && row.is_valid
                    && row.major == old.major
                    && JavaBuildVersion::parse(&row.full_version).as_ref() == Some(&latest.version)
            });

        let new = match installed {
            Some(row) => row,
            None => {
                let new_id = self
                    .managed_service
                    .setup_managed(vendor, latest.id.clone(), self.app.clone(), None)
                    .await?;

                java_repo::get_java_by_id(&self.app.db, &new_id)
                    .await?
                    .ok_or_else(|| anyhow::anyhow!("The installed Java is missing"))?
            }
        };

        if new.id == old.id || new.path == old.path {
            anyhow::bail!("Java {} is already up to date", old.full_version);
        }

        // So a row's Delete, Remove unused or the deferred-delete pass can't
        // delete the old Java while this replaces it. One may have gone first
        // while the new build installed.
        let _guard = self.removal_lock.lock().await;
        if java_repo::get_java_by_id(&self.app.db, &old.id)
            .await?
            .is_none()
        {
            anyhow::bail!("Java {} was deleted while it updated", old.full_version);
        }
        self.replace_managed_java(&old, &new).await?;
        self.app.invalidate(GET_MANAGED_JAVA_UPDATES, None);

        Ok(new.id)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::managers::java::managed::BuildSource;

    fn row(id: &str, ty: &str, major: i32, version: &str, valid: bool) -> JavaRow {
        JavaRow {
            id: id.into(),
            path: format!("/x/managed_javas/zulu-{id}/bin/java"),
            major,
            full_version: version.into(),
            r#type: ty.into(),
            os: "linux".into(),
            arch: "amd64".into(),
            vendor: "azul".into(),
            is_valid: valid,
        }
    }

    fn build(id: &str, major: u8, version: &str) -> ManagedJavaBuild {
        ManagedJavaBuild {
            id: id.into(),
            name: format!("zulu{version}"),
            major,
            version: JavaBuildVersion::parse(version).unwrap(),
            source: BuildSource::AzulZip {
                download_url: String::new(),
                sha256: None,
            },
            size_bytes: None,
            release_date: None,
            lts: false,
        }
    }

    fn catalog() -> ManagedJavaCatalog {
        ManagedJavaCatalog::from_builds(vec![
            build("b17-old", 17, "17.0.20.1"),
            build("b17-new", 17, "17.0.21.1"),
        ])
    }

    fn find_updates(
        rows: &[JavaRow],
        catalogs: &HashMap<JavaVendor, ManagedJavaCatalog>,
    ) -> Vec<ManagedJavaUpdate> {
        find_managed_updates(rows, catalogs, Path::new("/x/managed_javas"))
    }

    #[test]
    fn mojang_java_whose_binary_reports_a_shorter_version_gets_no_update() {
        // Mojang lists alpha as 16.0.1.9.1, while its binary says 16.0.1.
        let mut r = row("a", "managed", 16, "16.0.1", true);
        r.path = "/x/managed_javas/mojang-java-runtime-alpha-16.0.1.9.1/bin/java".into();
        let mut alpha = build("m", 16, "16.0.1.9.1");
        alpha.name = "java-runtime-alpha".into();
        alpha.source = BuildSource::MojangManifest {
            component: "java-runtime-alpha".into(),
            manifest_url: String::new(),
            manifest_sha1: String::new(),
        };
        let catalogs = HashMap::from([(
            JavaVendor::Mojang,
            ManagedJavaCatalog::from_builds(vec![alpha]),
        )]);

        assert!(find_updates(&[r], &catalogs).is_empty());
    }

    #[test]
    fn a_zulu_home_folder_does_not_make_a_mojang_java_azul() {
        let mut r = row("a", "managed", 17, "17.0.15", true);
        r.path = "/home/zuluaga/managed_javas/mojang-java-runtime-gamma-17.0.15/bin/java".into();

        assert!(find_updates(&[r], &azul(catalog())).is_empty());
    }

    fn azul(catalog: ManagedJavaCatalog) -> HashMap<JavaVendor, ManagedJavaCatalog> {
        HashMap::from([(JavaVendor::Azul, catalog)])
    }

    #[test]
    fn older_managed_gets_update() {
        let u = find_updates(
            &[row("a", "managed", 17, "17.0.20.1", true)],
            &azul(catalog()),
        );
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].java_id, "a");
        assert_eq!(
            u[0].latest.version,
            JavaBuildVersion::parse("17.0.21.1").unwrap()
        );
    }

    fn mojang_row(id: &str, version: &str) -> JavaRow {
        let mut r = row(id, "managed", 17, version, true);
        r.path = format!("/x/managed_javas/mojang-java-runtime-gamma-{version}/bin/java");
        r
    }

    #[test]
    fn updates_stay_within_distribution() {
        let mojang_17_15 = ManagedJavaCatalog::from_builds(vec![build("m", 17, "17.0.15")]);
        let mojang_17_21 = ManagedJavaCatalog::from_builds(vec![build("m", 17, "17.0.21.1")]);
        let rows = [mojang_row("a", "17.0.15")];

        let same = HashMap::from([
            (JavaVendor::Azul, catalog()),
            (JavaVendor::Mojang, mojang_17_15),
        ]);
        assert!(find_updates(&rows, &same).is_empty());

        let newer = HashMap::from([
            (JavaVendor::Azul, catalog()),
            (JavaVendor::Mojang, mojang_17_21),
        ]);
        let u = find_updates(&rows, &newer);
        assert_eq!(u.len(), 1);
        assert_eq!(u[0].vendor, JavaVendor::Mojang);
        assert_eq!(u[0].latest.id, "m");
    }

    #[test]
    fn row_of_unknown_distribution_is_skipped() {
        let mut r = row("a", "managed", 17, "17.0.20.1", true);
        r.path = "/opt/jdk17/bin/java".into();
        assert!(find_updates(&[r], &azul(catalog())).is_empty());
    }

    #[test]
    fn up_to_date_gets_none() {
        let u = find_updates(
            &[row("a", "managed", 17, "17.0.21.1", true)],
            &azul(catalog()),
        );
        assert!(u.is_empty());
    }

    #[test]
    fn local_is_ignored() {
        let u = find_updates(
            &[row("a", "local", 17, "17.0.20.1", true)],
            &azul(catalog()),
        );
        assert!(u.is_empty());
    }

    #[test]
    fn invalid_is_ignored() {
        let u = find_updates(
            &[row("a", "managed", 17, "17.0.20.1", false)],
            &azul(catalog()),
        );
        assert!(u.is_empty());
    }

    #[test]
    fn major_missing_from_catalog_gets_none() {
        let u = find_updates(&[row("a", "managed", 23, "23.0.2", true)], &azul(catalog()));
        assert!(u.is_empty());
    }

    #[test]
    fn unparsable_version_is_skipped() {
        let u = find_updates(
            &[row("a", "managed", 17, "garbage", true)],
            &azul(catalog()),
        );
        assert!(u.is_empty());
    }

    #[test]
    fn two_rows_share_latest() {
        let rows = [
            row("a", "managed", 17, "17.0.19", true),
            row("b", "managed", 17, "17.0.20.1", true),
        ];
        let u = find_updates(&rows, &azul(catalog()));
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].latest.id, u[1].latest.id);
    }

    use crate::{
        domain::{
            instance::{InstanceId, info},
            java::SystemJavaProfileName,
        },
        managers::{
            instance::{InstanceType, InstanceVersionSource},
            java::managed::should_pin,
        },
        setup_managers_for_test,
    };

    fn managed_row(app: &crate::App, id: &str, dir: &str, version: &str) -> JavaRow {
        let root = app
            .settings_manager()
            .runtime_path
            .get_managed_javas()
            .to_path();
        let bin_dir = root.join(dir).join("bin");
        std::fs::create_dir_all(&bin_dir).unwrap();
        let bin = bin_dir.join("java");
        std::fs::write(&bin, b"").unwrap();

        JavaRow {
            id: id.into(),
            path: bin.to_string_lossy().into_owned(),
            major: 17,
            full_version: version.into(),
            r#type: "managed".into(),
            os: "linux".into(),
            arch: "amd64".into(),
            vendor: "azul".into(),
            is_valid: true,
        }
    }

    pub(crate) async fn make_instance(
        app: &crate::App,
        name: &str,
        java_override: info::JavaOverride,
    ) -> InstanceId {
        let manager = app.instance_manager();
        let group = manager.get_default_group().await.unwrap();
        let id = manager
            .create_instance(
                group,
                name.into(),
                false,
                InstanceVersionSource::Version(info::GameVersion::Standard(
                    info::StandardVersion {
                        release: "1.20.1".into(),
                        modloaders: Default::default(),
                    },
                )),
                String::new(),
            )
            .await
            .unwrap();
        manager
            .update_instance(crate::domain::instance::InstanceSettingsUpdate {
                instance_id: id,
                name: None,
                use_loaded_icon: None,
                notes: None,
                version: None,
                modloader: None,
                java_override: Some(Some(java_override)),
                java_floor: None,
                global_java_args: None,
                extra_java_args: None,
                memory: None,
                pre_launch_hook: None,
                post_exit_hook: None,
                wrapper_command: None,
                game_resolution: None,
                mod_sources: None,
                modpack_locked: None,
            })
            .await
            .unwrap();
        id
    }

    pub(crate) async fn override_of(
        app: &crate::App,
        id: InstanceId,
    ) -> Option<info::JavaOverride> {
        let manager = app.instance_manager();
        let instances = manager.instances.read().await;
        let InstanceType::Valid(data) = &instances.get(&id).unwrap().type_ else {
            panic!("invalid instance");
        };
        data.java_override().cloned()
    }

    pub(crate) async fn setup_old_and_new(app: &crate::App) -> (JavaRow, JavaRow) {
        let old = managed_row(app, "old-id", "zulu17.0.19-old", "17.0.19");
        let new = managed_row(app, "new-id", "zulu17.0.21-new", "17.0.21.1");
        java_repo::insert_java(&app.db, old.clone()).await.unwrap();
        java_repo::insert_java(&app.db, new.clone()).await.unwrap();
        java_repo::upsert_profile(&app.db, "gamma", true)
            .await
            .unwrap();
        java_repo::set_profile_java(&app.db, "gamma", Some("old-id"))
            .await
            .unwrap();
        (old, new)
    }

    #[tokio::test]
    async fn replace_repoints_profiles_and_path_overrides_and_removes_old() {
        let app = setup_managers_for_test().await;
        let (old, new) = setup_old_and_new(&app.app).await;

        let by_path = make_instance(
            &app.app,
            "by-path",
            info::JavaOverride::Path(Some(old.path.clone())),
        )
        .await;
        let by_profile = make_instance(
            &app.app,
            "by-profile",
            info::JavaOverride::Profile(Some("gamma".into())),
        )
        .await;

        let outcome = app
            .java_manager()
            .replace_managed_java(&old, &new)
            .await
            .unwrap();

        assert_eq!(outcome, ReplaceOutcome::Removed);
        let gamma = java_repo::get_profile(&app.db, "gamma")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(gamma.java_id.as_deref(), Some("new-id"));
        assert!(matches!(
            override_of(&app.app, by_path).await,
            Some(info::JavaOverride::Path(Some(p))) if p == new.path
        ));
        assert!(matches!(
            override_of(&app.app, by_profile).await,
            Some(info::JavaOverride::Profile(Some(p))) if p == "gamma"
        ));
        assert!(
            java_repo::get_java_by_id(&app.db, "old-id")
                .await
                .unwrap()
                .is_none()
        );
        assert!(!std::path::Path::new(&old.path).exists());
        assert!(std::path::Path::new(&new.path).exists());
    }

    #[tokio::test]
    async fn replace_defers_removal_while_an_instance_runs() {
        let app = setup_managers_for_test().await;
        let (old, new) = setup_old_and_new(&app.app).await;
        let manager = app.instance_manager();
        manager.any_instance_running.send_replace(true);

        let outcome = app
            .java_manager()
            .replace_managed_java(&old, &new)
            .await
            .unwrap();

        assert_eq!(outcome, ReplaceOutcome::Deferred);
        assert!(
            java_repo::get_java_by_id(&app.db, "old-id")
                .await
                .unwrap()
                .is_some()
        );
        assert!(std::path::Path::new(&old.path).exists());
        let gamma = java_repo::get_profile(&app.db, "gamma")
            .await
            .unwrap()
            .unwrap();
        assert_eq!(gamma.java_id.as_deref(), Some("new-id"));

        // Still running: nothing is deleted.
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            0
        );
        assert!(std::path::Path::new(&old.path).exists());

        manager.any_instance_running.send_replace(false);
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            1
        );
        assert!(
            java_repo::get_java_by_id(&app.db, "old-id")
                .await
                .unwrap()
                .is_none()
        );
        assert!(!std::path::Path::new(&old.path).exists());
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            0
        );
    }

    #[tokio::test]
    async fn replace_defers_removal_while_a_server_runs() {
        use crate::domain::{server::ServerId, vtask::VisualTaskId};
        use crate::managers::server::ServerData;

        let app = setup_managers_for_test().await;
        let (old, new) = setup_old_and_new(&app.app).await;
        let set_state = |state| {
            let app = app.app.clone();
            async move {
                app.server_manager().servers.write().await.insert(
                    ServerId(1),
                    ServerData {
                        shortpath: "srv".into(),
                        state,
                        handle: None,
                        last_log_id: None,
                        java_path: None,
                    },
                );
            }
        };
        set_state(crate::domain::server::ServerState::Starting(VisualTaskId(
            1,
        )))
        .await;

        let outcome = app
            .java_manager()
            .replace_managed_java(&old, &new)
            .await
            .unwrap();

        assert_eq!(outcome, ReplaceOutcome::Deferred);
        assert!(std::path::Path::new(&old.path).exists());
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            0
        );

        set_state(crate::domain::server::ServerState::Stopped { failed_task: None }).await;
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            1
        );
        assert!(!std::path::Path::new(&old.path).exists());
    }

    #[tokio::test]
    async fn a_pending_removal_that_is_already_gone_is_a_no_op() {
        let app = setup_managers_for_test().await;
        app.java_manager()
            .pending_removals
            .lock()
            .await
            .insert("gone".into());

        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            0
        );
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            0
        );
        assert!(app.java_manager().pending_removals.lock().await.is_empty());
    }

    /// Reading the version data may wait on the network, and a replace must
    /// not wait behind it to add its Java to the pending list.
    #[tokio::test]
    async fn the_pending_list_is_free_while_the_version_data_loads() {
        let app = setup_managers_for_test().await;
        let manager = app.java_manager();
        manager.pending_removals.lock().await.insert("gone".into());
        // Holds up the version data until released.
        let loading = manager.version_profiles.write().await;

        let free = async {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let free = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                manager.pending_removals.lock(),
            )
            .await
            .is_ok();
            drop(loading);
            free
        };
        let (removed, free) = tokio::join!(manager.process_pending_removals(), free);

        assert!(
            free,
            "the pending list was held while the version data loaded"
        );
        assert_eq!(removed.unwrap(), 0);
    }

    #[tokio::test]
    async fn replace_defers_while_a_launch_is_preparing() {
        let app = setup_managers_for_test().await;
        let (old, new) = setup_old_and_new(&app.app).await;
        let id = make_instance(
            &app.app,
            "launching",
            info::JavaOverride::Path(Some(old.path.clone())),
        )
        .await;
        app.instance_manager()
            .set_launch_state_for_test(
                id,
                crate::managers::instance::run::LaunchState::Preparing(
                    crate::domain::vtask::VisualTaskId(1),
                ),
            )
            .await;

        let outcome = app
            .java_manager()
            .replace_managed_java(&old, &new)
            .await
            .unwrap();

        assert_eq!(outcome, ReplaceOutcome::Deferred);
        assert!(std::path::Path::new(&old.path).exists());
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            0
        );

        app.instance_manager()
            .set_launch_state_for_test(
                id,
                crate::managers::instance::run::LaunchState::Inactive { failed_task: None },
            )
            .await;
        assert_eq!(
            app.java_manager().process_pending_removals().await.unwrap(),
            1
        );
        assert!(!std::path::Path::new(&old.path).exists());
    }

    #[tokio::test]
    async fn replace_keeps_old_java_for_profile_that_cannot_use_new() {
        let app = setup_managers_for_test().await;
        let mut old = managed_row(&app.app, "old8", "zulu8.0.302-old", "8.0.302");
        let mut new = managed_row(&app.app, "new8", "zulu8.0.492-new", "8.0.492");
        old.major = 8;
        new.major = 8;
        java_repo::insert_java(&app.db, old.clone()).await.unwrap();
        java_repo::insert_java(&app.db, new.clone()).await.unwrap();

        let legacy = SystemJavaProfileName::Legacy.to_string();
        let fixed = SystemJavaProfileName::LegacyFixed1.to_string();
        for name in [&legacy, &fixed] {
            java_repo::upsert_profile(&app.db, name, true)
                .await
                .unwrap();
            java_repo::set_profile_java(&app.db, name, Some("old8"))
                .await
                .unwrap();
        }

        let outcome = app
            .java_manager()
            .replace_managed_java(&old, &new)
            .await
            .unwrap();

        assert_eq!(outcome, ReplaceOutcome::Kept);
        let java_of = |name: String| {
            let db = app.db.clone();
            async move {
                java_repo::get_profile(&db, &name)
                    .await
                    .unwrap()
                    .unwrap()
                    .java_id
            }
        };
        assert_eq!(java_of(legacy).await.as_deref(), Some("new8"));
        assert_eq!(java_of(fixed).await.as_deref(), Some("old8"));
        assert!(
            java_repo::get_java_by_id(&app.db, "old8")
                .await
                .unwrap()
                .is_some()
        );
        assert!(std::path::Path::new(&old.path).exists());
        assert!(
            get_pinned_managed_javas(&app.db)
                .await
                .unwrap()
                .contains("old8")
        );
        assert!(app.java_manager().pending_removals.lock().await.is_empty());
    }

    #[tokio::test]
    async fn replacing_a_pinned_java_unpins_it() {
        let app = setup_managers_for_test().await;
        let (old, new) = setup_old_and_new(&app.app).await;
        set_pinned_managed_javas(
            &app.db,
            &HashSet::from(["old-id".to_string(), "x".to_string()]),
        )
        .await
        .unwrap();

        app.java_manager()
            .replace_managed_java(&old, &new)
            .await
            .unwrap();

        assert_eq!(
            get_pinned_managed_javas(&app.db).await.unwrap(),
            HashSet::from(["x".to_string()])
        );
    }

    #[tokio::test]
    async fn pinned_set_reads_empty_when_missing_or_garbage() {
        let app = setup_managers_for_test().await;
        assert!(get_pinned_managed_javas(&app.db).await.unwrap().is_empty());

        frontend_preference::upsert_preference(
            &app.db,
            MANAGED_JAVA_PINNED_KEY,
            "not json",
            DbDateTime(chrono::Utc::now().fixed_offset()),
        )
        .await
        .unwrap();
        assert!(get_pinned_managed_javas(&app.db).await.unwrap().is_empty());
    }

    fn update(id: &str, major: u8, current: &str, latest: &str) -> ManagedJavaUpdate {
        ManagedJavaUpdate {
            java_id: id.into(),
            vendor: JavaVendor::Azul,
            current: JavaBuildVersion::parse(current).unwrap(),
            latest: build(&format!("latest-{id}"), major, latest),
        }
    }

    fn profile(name: &str, java_id: &str) -> JavaProfileRow {
        JavaProfileRow {
            name: name.into(),
            is_system_profile: true,
            java_id: Some(java_id.into()),
        }
    }

    #[test]
    fn auto_update_off_applies_nothing() {
        let updates = [update("a", 17, "17.0.19", "17.0.21.1")];
        assert!(updates_to_apply(&updates, false, &HashSet::new()).is_empty());
    }

    #[test]
    fn pinned_java_is_not_applied() {
        let updates = [
            update("a", 17, "17.0.19", "17.0.21.1"),
            update("b", 17, "17.0.20", "17.0.21.1"),
        ];
        let pinned = HashSet::from(["a".to_string()]);

        let applied = updates_to_apply(&updates, true, &pinned);

        assert_eq!(
            applied
                .iter()
                .map(|u| u.java_id.as_str())
                .collect::<Vec<_>>(),
            vec!["b"]
        );
    }

    #[test]
    fn every_update_is_applied_by_default() {
        let updates = [
            update("a", 17, "17.0.19", "17.0.21.1"),
            update("b", 17, "17.0.20", "17.0.21.1"),
        ];
        assert_eq!(updates_to_apply(&updates, true, &HashSet::new()).len(), 2);
    }

    #[test]
    fn updates_skip_javas_waiting_for_a_deferred_delete() {
        let updates = vec![
            update("a", 17, "17.0.19", "17.0.21.1"),
            update("b", 17, "17.0.20", "17.0.21.1"),
        ];
        let pending = HashSet::from(["a".to_string()]);

        let kept = without_looping_updates(updates, &[], &pending);

        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].java_id, "b");
    }

    #[test]
    fn updates_skip_javas_a_profile_could_not_move_off() {
        let fixed = SystemJavaProfileName::LegacyFixed1.to_string();
        let legacy = SystemJavaProfileName::Legacy.to_string();
        // Java 8 below update 312 is all `legacy_fixed_1` accepts.
        let updates = vec![
            update("kept", 8, "8.0.302", "8.0.492"),
            update("free", 8, "8.0.302", "8.0.492"),
            update("fixed-ok", 8, "8.0.302", "8.0.305"),
        ];
        let profiles = [
            profile(&fixed, "kept"),
            profile(&legacy, "free"),
            profile(&fixed, "fixed-ok"),
        ];

        let kept = without_looping_updates(updates, &profiles, &HashSet::new());

        assert_eq!(
            kept.iter().map(|u| u.java_id.as_str()).collect::<Vec<_>>(),
            vec!["free", "fixed-ok"]
        );
    }

    #[tokio::test]
    async fn update_managed_java_rejects_a_java_that_is_not_managed() {
        let app = setup_managers_for_test().await;
        let mut local = managed_row(&app.app, "loc", "zulu17.0.19-loc", "17.0.19");
        local.r#type = "local".into();
        java_repo::insert_java(&app.db, local).await.unwrap();

        let err = app
            .java_manager()
            .update_managed_java("loc")
            .await
            .unwrap_err();

        assert!(err.to_string().contains("not managed"), "{err:#}");
        assert!(
            app.java_manager()
                .update_managed_java("missing")
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn installing_non_latest_build_pins_it() {
        let catalog = catalog();

        assert!(should_pin(&catalog, &build("b17-19", 17, "17.0.19")));
        assert!(!should_pin(&catalog, &build("b17-new", 17, "17.0.21.1")));
    }

    async fn fixable(
        app: &crate::App,
        catalog: Option<&ManagedJavaCatalog>,
        id: &str,
        fixed: &str,
    ) -> bool {
        app.java_manager()
            .auto_fixable_ids(
                &catalog.map(|c| azul(c.clone())).unwrap_or_default(),
                &[(id.to_string(), JavaBuildVersion::parse(fixed))],
            )
            .await
            .unwrap()
            .contains(id)
    }

    async fn fixable_setup() -> (crate::TestEnv, ManagedJavaCatalog) {
        let app = setup_managers_for_test().await;
        let row = managed_row(&app.app, "j17", "zulu17.0.19-old", "17.0.19");
        java_repo::insert_java(&app.db, row).await.unwrap();
        let catalog = ManagedJavaCatalog::from_builds(vec![build("b17", 17, "17.0.21.1")]);
        (app, catalog)
    }

    #[tokio::test]
    async fn auto_fixable_when_updater_would_fix_it() {
        let (app, catalog) = fixable_setup().await;
        assert!(fixable(&app.app, Some(&catalog), "j17", "17.0.21.1").await);
        assert!(!fixable(&app.app, Some(&catalog), "j17", "17.0.22").await);
    }

    #[tokio::test]
    async fn auto_fixable_false_when_pinned() {
        let (app, catalog) = fixable_setup().await;
        set_pinned_managed_javas(&app.db, &HashSet::from(["j17".to_string()]))
            .await
            .unwrap();
        assert!(!fixable(&app.app, Some(&catalog), "j17", "17.0.21.1").await);
    }

    #[tokio::test]
    async fn auto_fixable_false_when_auto_update_is_off() {
        let (app, catalog) = fixable_setup().await;
        app.db
            .write(|conn| {
                let patch = carbon_repos::repos::app_configuration::AppConfigurationPatch {
                    auto_update_managed_java: Some(false),
                    ..Default::default()
                };
                Ok(patch.build().map(|q| q.execute(&conn)).transpose()?)
            })
            .await
            .unwrap();
        assert!(!fixable(&app.app, Some(&catalog), "j17", "17.0.21.1").await);
    }

    #[tokio::test]
    async fn auto_fixable_false_without_catalog() {
        let (app, _) = fixable_setup().await;
        assert!(!fixable(&app.app, None, "j17", "17.0.21.1").await);
    }

    #[tokio::test]
    async fn auto_fixable_false_for_pending_removal() {
        let (app, catalog) = fixable_setup().await;
        app.java_manager()
            .pending_removals
            .lock()
            .await
            .insert("j17".into());
        assert!(!fixable(&app.app, Some(&catalog), "j17", "17.0.21.1").await);
    }

    #[tokio::test]
    async fn auto_fixable_false_for_legacy_fixed_profile_java() {
        let app = setup_managers_for_test().await;
        let mut row = managed_row(&app.app, "j8", "zulu8.0.302-old", "8.0.302");
        row.major = 8;
        java_repo::insert_java(&app.db, row).await.unwrap();
        let name = SystemJavaProfileName::LegacyFixed1.to_string();
        java_repo::upsert_profile(&app.db, &name, true)
            .await
            .unwrap();
        java_repo::set_profile_java(&app.db, &name, Some("j8"))
            .await
            .unwrap();
        let catalog = ManagedJavaCatalog::from_builds(vec![build("b8", 8, "8.0.492")]);

        assert!(!fixable(&app.app, Some(&catalog), "j8", "8.0.492").await);
    }

    #[tokio::test]
    async fn replace_keeps_old_java_for_a_forge_1_16_5_path_override() {
        use crate::managers::java::test_support::add_instance;

        let app = setup_managers_for_test().await;
        let mut old = managed_row(&app.app, "old8", "zulu8.0.302-old", "8.0.302");
        let mut new = managed_row(&app.app, "new8", "zulu8.0.492-new", "8.0.492");
        old.major = 8;
        new.major = 8;
        java_repo::insert_java(&app.db, old.clone()).await.unwrap();
        java_repo::insert_java(&app.db, new.clone()).await.unwrap();
        let path = Some(info::JavaOverride::Path(Some(old.path.clone())));
        let forge = add_instance(&app.app, "Forge", "1.16.5", true, path.clone()).await;
        let vanilla = add_instance(&app.app, "Vanilla", "1.16.5", false, path).await;

        let outcome = app
            .java_manager()
            .replace_managed_java(&old, &new)
            .await
            .unwrap();

        assert_eq!(outcome, ReplaceOutcome::Kept);
        assert!(matches!(
            override_of(&app.app, forge).await,
            Some(info::JavaOverride::Path(Some(p))) if p == old.path
        ));
        assert!(matches!(
            override_of(&app.app, vanilla).await,
            Some(info::JavaOverride::Path(Some(p))) if p == new.path
        ));
        assert!(std::path::Path::new(&old.path).exists());
        assert!(
            get_pinned_managed_javas(&app.db)
                .await
                .unwrap()
                .contains("old8")
        );
    }
}
