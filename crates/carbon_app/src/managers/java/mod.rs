use self::{
    discovery::Discovery,
    java_checker::JavaChecker,
    managed::{ManagedJavaCatalog, ManagedService, Step, pick_auto_install_build},
    resolve::{JavaRequirement, link_requirement_profiles, requirement_for},
};
use super::ManagerRef;
use crate::{
    api::keys::{
        instance::INSTANCE_DETAILS,
        java::{
            GET_AVAILABLE_JAVAS, GET_JAVA_PROFILES, GET_MANAGED_JAVA_UPDATES,
            SYSTEM_JAVA_PROFILE_ASSIGNMENTS,
        },
    },
    domain::{
        instance::{InstanceId, info::StandardVersion},
        java::{
            Java, JavaArch, JavaComponent, JavaComponentType, JavaOs, JavaProfile, JavaVendor,
            SYSTEM_JAVA_PROFILE_NAME_PREFIX, SystemJavaProfileName,
        },
    },
    managers::java::java_checker::RealJavaChecker,
};
use anyhow::bail;
use carbon_repos::db_exec::Db;
use carbon_repos::repos::java as java_repo;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, watch};
use tracing::debug;

pub mod advisories;
pub mod build_version;
mod constants;
pub mod discovery;
pub mod instance_java;
pub mod java_checker;
pub mod java_display;
pub mod maintenance;
pub mod major_defaults;
pub mod managed;
pub mod minecraft_majors;
pub mod overview;
pub mod overview_actions;
mod parser;
pub mod resolve;
pub mod scan_and_sync;
#[cfg(test)]
pub(crate) mod test_support;
pub mod updates;
pub mod utils;

pub(crate) struct JavaManager {
    pub managed_service: ManagedService,
    /// Managed Java ids to delete once no game is running.
    pub pending_removals: tokio::sync::Mutex<std::collections::HashSet<String>>,
    /// Ids of Javas the user deleted while a game or server ran. Unlike
    /// `pending_removals` they go whatever uses them then: the user confirmed
    /// that their instances move to Automatic.
    pub requested_removals: tokio::sync::Mutex<std::collections::HashSet<String>>,
    /// Held while deferred deletes run, so two callers never delete the same Java.
    pub removal_lock: tokio::sync::Mutex<()>,
    /// From the last successful advisory fetch of this session; empty before.
    pub advisories: tokio::sync::RwLock<Vec<advisories::JavaAdvisory>>,
    /// Install folder sizes of managed Javas, by Java id, computed once.
    pub install_sizes: tokio::sync::Mutex<HashMap<String, u64>>,
    /// The last good Minecraft version profiles of this session.
    pub(crate) version_profiles:
        tokio::sync::RwLock<Option<minecraft_majors::CachedVersionProfiles>>,
    /// The Java binary each instance's last launch this session started on.
    /// A game keeps running on it even when its instance moves to another
    /// Java meanwhile.
    launch_javas: std::sync::Mutex<HashMap<InstanceId, String>>,
    /// Told each time a game or server stops, so the deletes waiting for it
    /// can go (see `maintenance::delete_removals_on_exit`).
    pub(crate) runner_ended: tokio::sync::Notify,
}

impl JavaManager {
    pub fn new() -> Self {
        Self {
            managed_service: ManagedService::new(),
            pending_removals: Default::default(),
            requested_removals: Default::default(),
            removal_lock: Default::default(),
            advisories: Default::default(),
            install_sizes: Default::default(),
            version_profiles: Default::default(),
            launch_javas: Default::default(),
            runner_ended: Default::default(),
        }
    }

    /// A game or server stopped: the deletes that waited for it may go.
    pub fn note_runner_ended(&self) {
        self.runner_ended.notify_one();
    }

    /// Records that `instance_id`'s game starts on the Java at `path`.
    pub fn note_launch_java(&self, instance_id: InstanceId, path: &str) {
        self.launch_javas
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(instance_id, path.to_string());
    }

    /// The Java binary `instance_id`'s last launch this session started on.
    pub(crate) fn launch_java(&self, instance_id: InstanceId) -> Option<String> {
        self.launch_javas
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&instance_id)
            .cloned()
    }

    /// Every Java waiting to be deleted once no game runs, for whatever
    /// reason.
    pub async fn removal_pending_ids(&self) -> std::collections::HashSet<String> {
        let mut ids = self.pending_removals.lock().await.clone();
        ids.extend(self.requested_removals.lock().await.iter().cloned());
        ids
    }

    pub async fn ensure_profiles_in_db(db: &Db) -> anyhow::Result<()> {
        debug!("Ensuring system java profiles are in db");
        // Interleaved app logic: iterates the SystemJavaProfileName domain enum
        // (a carbon_app type) and seeds every profile, so it stays an explicit
        // closure with `_conn` forms. The upserts run in one writer dispatch,
        // so no other write interleaves; they run in ONE transaction —
        // all-or-nothing: a failure rolls the whole group back and readers
        // never observe an intermediate state. `_conn` forms on the tx guard.
        db.write(|mut conn| {
            let tx = conn.transaction()?;
            for profile in SystemJavaProfileName::iter() {
                java_repo::upsert_profile_conn(&tx, &profile.to_string(), true)?;
            }
            tx.commit()?;
            Ok(())
        })
        .await?;

        Ok(())
    }

    /// Scans for Javas, then, with `link_profiles`, links the system profiles
    /// to their automatic choices. Startup passes `false` until the defaults
    /// migration has run, so the Javas picked by hand stay linked for it to
    /// read.
    pub async fn scan_and_sync<T, G>(
        db: &Db,
        discovery: &T,
        java_checker: &G,
        link_profiles: bool,
    ) -> anyhow::Result<()>
    where
        T: Discovery,
        G: JavaChecker,
    {
        let t = std::time::Instant::now();
        scan_and_sync::scan_and_sync_local(db, discovery, java_checker).await?;
        tracing::debug!(
            "[startup-timing] scan_and_sync_local completed in {:.2}s",
            t.elapsed().as_secs_f64()
        );

        let t = std::time::Instant::now();
        scan_and_sync::scan_and_sync_custom(db, java_checker).await?;
        tracing::debug!(
            "[startup-timing] scan_and_sync_custom completed in {:.2}s",
            t.elapsed().as_secs_f64()
        );

        let t = std::time::Instant::now();
        scan_and_sync::scan_and_sync_managed(db, discovery, java_checker).await?;
        tracing::debug!(
            "[startup-timing] scan_and_sync_managed completed in {:.2}s",
            t.elapsed().as_secs_f64()
        );

        if !link_profiles {
            return Ok(());
        }

        let t = std::time::Instant::now();
        // No advisories are fetched yet at startup, so nothing is flagged.
        scan_and_sync::sync_system_java_profiles(db, &Default::default()).await?;
        tracing::debug!(
            "[startup-timing] sync_system_java_profiles completed in {:.2}s",
            t.elapsed().as_secs_f64()
        );

        Ok(())
    }
}

impl ManagerRef<'_, JavaManager> {
    /// Whether a game or a launcher-hosted server is starting or running, so
    /// a managed Java must not be deleted.
    pub async fn java_in_use_by_running(&self) -> bool {
        self.app.instance_manager().any_launch_active().await
            || self.app.server_manager().any_server_active().await
    }

    pub async fn get_available_javas(&self) -> anyhow::Result<HashMap<u8, Vec<Java>>> {
        let all_javas = java_repo::get_all_java(&self.app.db).await?;

        let mut result = HashMap::new();

        for java in all_javas {
            let major_version = java.major as u8;
            let javas = result.entry(major_version).or_insert_with(Vec::new);
            javas.push(Java::try_from(java)?);
        }

        Ok(result)
    }

    pub async fn get_java_profiles(&self) -> anyhow::Result<Vec<JavaProfile>> {
        let all_profiles = java_repo::get_all_profiles(&self.app.db)
            .await?
            .into_iter()
            .map(JavaProfile::try_from)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(all_profiles)
    }

    pub async fn validate_custom_java_path(&self, path: String) -> anyhow::Result<bool> {
        let p = Path::new(&path);

        // check if file is executable
        if !p.is_file() {
            return Ok(false);
        }

        let java = RealJavaChecker::get_bin_info(
            &RealJavaChecker::new(self.app.runtime_lifecycle.clone()),
            p,
            JavaComponentType::Custom,
        )
        .await
        .is_ok();

        Ok(java)
    }

    pub async fn update_java_profile(
        &self,
        profile_name: String,
        java_id: Option<String>,
    ) -> anyhow::Result<()> {
        refuse_system_profile(&profile_name)?;

        if let Some(java_id) = java_id {
            java_repo::set_profile_java(&self.app.db, &profile_name, Some(&java_id)).await?;
        } else {
            java_repo::set_profile_java(&self.app.db, &profile_name, None).await?;
        }

        self.app.invalidate(GET_JAVA_PROFILES, None);

        Ok(())
    }

    pub async fn create_java_profile(
        &self,
        profile_name: String,
        java_id: Option<String>,
    ) -> anyhow::Result<()> {
        create_custom_profile(&self.app.db, profile_name, java_id).await?;

        self.app.invalidate(GET_JAVA_PROFILES, None);

        Ok(())
    }

    pub async fn delete_java_profile(&self, profile_name: String) -> anyhow::Result<()> {
        refuse_system_profile(&profile_name)?;

        java_repo::delete_profile(&self.app.db, &profile_name).await?;

        self.app.invalidate(GET_JAVA_PROFILES, None);

        Ok(())
    }

    pub async fn create_custom_java_version(&self, path: String) -> anyhow::Result<()> {
        let activity = self.app.runtime_lifecycle.admit_owned(
            "custom Java registration",
            crate::managers::runtime_lifecycle::ActivityPolicy::JavaProbe,
        )?;
        let app = self.app.clone();
        crate::managers::runtime_lifecycle::spawn_retained(activity, move |_| {
            Box::pin(async move {
                app.java_manager()
                    .create_custom_java_version_owned(path)
                    .await
            })
        })
        .await?
    }

    async fn create_custom_java_version_owned(&self, path: String) -> anyhow::Result<()> {
        let java = RealJavaChecker::get_bin_info(
            &RealJavaChecker::new(self.app.runtime_lifecycle.clone()),
            Path::new(&path),
            JavaComponentType::Custom,
        )
        .await?;

        let path_for_lookup = path.clone();
        let exists = java_repo::get_java_by_path(&self.app.db, &path_for_lookup).await?;

        if exists.is_some() {
            anyhow::bail!("Java with path {} already exists", path);
        }

        let row = java_repo::JavaRow {
            id: uuid::Uuid::new_v4().to_string(),
            path: java.path,
            major: java.version.major as i32,
            full_version: java.version.to_string(),
            r#type: java._type.to_string(),
            os: java.os.to_string(),
            arch: java.arch.to_string(),
            vendor: java.vendor,
            is_valid: true,
        };
        java_repo::insert_java(&self.app.db, row).await?;

        self.app.invalidate(GET_AVAILABLE_JAVAS, None);

        Ok(())
    }

    pub async fn delete_java_version(&self, java_id: String) -> anyhow::Result<()> {
        let id_for_lookup = java_id.clone();
        let java_from_db = java_repo::get_java_by_id(&self.app.db, &id_for_lookup)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Java with id {} not found", java_id.clone()))?;

        let java_component_type = JavaComponentType::try_from(&*java_from_db.r#type)?;
        let deleted_path = java_from_db.path.clone();

        match java_component_type {
            JavaComponentType::Custom => {
                let id = java_id.clone();
                java_repo::delete_java(&self.app.db, &id).await?;
            }
            JavaComponentType::Managed => {
                let root_managed_path = self
                    .app
                    .settings_manager()
                    .runtime_path
                    .get_managed_javas()
                    .to_path();
                let java_bin_path = PathBuf::from(java_from_db.path);

                // A Java installed while a data folder on a network share was still
                // resolved to its `\\?\UNC\` form is stored under that form.
                let plain_java_bin_path =
                    crate::runtime_path_override::plain_share_path(&java_bin_path);
                let managed_java_dir_name = java_bin_path
                    .strip_prefix(&root_managed_path)
                    .or_else(|_| plain_java_bin_path.strip_prefix(&root_managed_path))?
                    .components()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Could not strip prefix"))?;

                let managed_java_dir = root_managed_path.join(managed_java_dir_name);

                if managed_java_dir.exists() {
                    std::fs::remove_dir_all(managed_java_dir)?;
                }

                let id = java_id.clone();
                java_repo::delete_java(&self.app.db, &id).await?;
            }
            JavaComponentType::Local => {
                anyhow::bail!("Java with id {} is local. Cannot delete.", java_id.clone());
            }
        }

        // Nothing may keep pointing at a deleted Java: its majors and the
        // instances set to its path go back to Automatic. Profiles are
        // unlinked by the database.
        major_defaults::clear_major_defaults_for(&self.app.db, &java_id).await?;
        self.app
            .instance_manager()
            .clear_java_path_overrides(&deleted_path)
            .await?;

        self.app.invalidate(GET_JAVA_PROFILES, None);
        self.app.invalidate(GET_AVAILABLE_JAVAS, None);

        Ok(())
    }

    /// Checks that the Java `id` runs and still meets `req`. `None` when it
    /// does not, so the caller resolves again:
    /// - a Java that fails to run is unlinked from every profile and marked
    ///   invalid;
    /// - one whose binary now reports another version has its row refreshed;
    /// - one that is gone is skipped.
    pub async fn check_java_for(
        &self,
        id: &str,
        req: &JavaRequirement,
    ) -> anyhow::Result<Option<JavaComponent>> {
        let Some(java) = java_repo::get_java_by_id(&self.app.db, id).await? else {
            return Ok(None);
        };

        let bin_info = RealJavaChecker::get_bin_info(
            &RealJavaChecker::new(self.app.runtime_lifecycle.clone()),
            Path::new(&java.path),
            (&*java.r#type).try_into()?,
        )
        .await;

        match bin_info {
            Ok(component) if req.accepts_component(&component) => Ok(Some(component)),
            Ok(component) => {
                tracing::warn!(
                    "Java {} now reports {}, refreshing it",
                    java.id,
                    component.version.to_string()
                );
                scan_and_sync::upsert_java_component_to_db(&self.app.db, component).await?;
                invalidate_java_lists(self.app);
                Ok(None)
            }
            Err(err) => {
                // Shutdown/cancel/unknown is not evidence that an installed Java is invalid.
                if self.app.runtime_lifecycle.admission_frozen()
                    || java_checker::validation_inconclusive(&err)
                {
                    return Err(err);
                }
                tracing::warn!(
                    "Java {} is not usable: {}. Marking it invalid",
                    java.id,
                    err
                );
                self.invalidate_java(java.id).await?;
                invalidate_java_lists(self.app);
                Ok(None)
            }
        }
    }

    /// Unlinks the Java from every profile and marks it invalid.
    async fn invalidate_java(&self, java_id: String) -> anyhow::Result<()> {
        let names_to_disconnect: Vec<String> = java_repo::get_all_profiles(&self.app.db)
            .await?
            .into_iter()
            .filter(|profile| profile.java_id.as_deref() == Some(java_id.as_str()))
            .map(|profile| profile.name)
            .collect();

        // Interleaved app logic: unlink the computed set of profiles then flip
        // the java's validity. Runs in one writer dispatch and ONE transaction
        // — all-or-nothing: a failure rolls the whole group back and readers
        // never observe an intermediate state. `_conn` forms on the tx guard.
        self.app
            .db
            .write(move |mut conn| {
                let tx = conn.transaction()?;
                for name in names_to_disconnect {
                    java_repo::set_profile_java_conn(&tx, &name, None)?;
                }
                java_repo::set_java_validity_conn(&tx, &java_id, false)?;
                tx.commit()?;
                Ok(())
            })
            .await?;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_usable_java_for_profile_name(
        self,
        target_profile: SystemJavaProfileName,
    ) -> anyhow::Result<Option<JavaComponent>> {
        let activity = self.app.runtime_lifecycle.admit_owned(
            "Java profile probe and publication",
            crate::managers::runtime_lifecycle::ActivityPolicy::JavaProbe,
        )?;
        let app = self.app.clone();
        crate::managers::runtime_lifecycle::spawn_retained(activity, move |_| {
            Box::pin(async move {
                app.java_manager()
                    .get_usable_java_for_profile_name_owned(target_profile)
                    .await
            })
        })
        .await?
    }

    async fn get_usable_java_for_profile_name_owned(
        self,
        target_profile: SystemJavaProfileName,
    ) -> anyhow::Result<Option<JavaComponent>> {
        let target_name = target_profile.to_string();
        let profile = java_repo::get_profile(&self.app.db, &target_name)
            .await?
            .filter(|profile| profile.is_system_profile)
            .ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

        let java = match profile.java_id {
            Some(java_id) => java_repo::get_java_by_id(&self.app.db, &java_id).await?,
            None => None,
        };

        let java = match java {
            Some(java) => {
                let bin_result = RealJavaChecker::get_bin_info(
                    &RealJavaChecker::new(self.app.runtime_lifecycle.clone()),
                    Path::new(&java.path),
                    (&*java.r#type).try_into()?,
                )
                .await;

                match bin_result {
                    Ok(bin_info) => Some(bin_info),
                    Err(err) => {
                        // Shutdown/cancel/unknown is not evidence that an installed Java is invalid.
                        if self.app.runtime_lifecycle.admission_frozen()
                            || java_checker::validation_inconclusive(&err)
                        {
                            return Err(err);
                        }
                        tracing::warn!(
                            "Java {} is not usable: {}. Cleaning it up from db",
                            java.id,
                            err
                        );
                        self.invalidate_java(java.id).await?;

                        None
                    }
                }
            }
            None => None,
        };

        Ok(java.and_then(|java| {
            if !target_profile.is_java_version_compatible(&java.version) {
                None
            } else {
                Some(java)
            }
        }))
    }

    /// Installs the newest Azul Zulu build that meets `req`. With
    /// `link_profiles`, every system profile of that requirement is linked to
    /// it. Fails with [`managed::AlreadyInstalling`] when another install of
    /// that build is running.
    pub async fn require_java_install(
        self,
        req: &JavaRequirement,
        link_profiles: bool,
        progress: Option<watch::Sender<Step>>,
    ) -> anyhow::Result<Option<JavaComponent>> {
        let activity = self.app.runtime_lifecycle.admit_owned(
            "Java installation and profile publication",
            crate::managers::runtime_lifecycle::ActivityPolicy::Installer,
        )?;
        let app = self.app.clone();
        let req = req.clone();
        crate::managers::runtime_lifecycle::spawn_retained(activity, move |slot| {
            Box::pin(async move {
                let cancel = slot.as_ref().unwrap().cancellation();
                app.java_manager()
                    .require_java_install_owned(&req, link_profiles, progress, cancel)
                    .await
            })
        })
        .await?
    }

    async fn require_java_install_owned(
        self,
        req: &JavaRequirement,
        link_profiles: bool,
        progress: Option<watch::Sender<Step>>,
        cancel: tokio_util::sync::CancellationToken,
    ) -> anyhow::Result<Option<JavaComponent>> {
        static LOCK: Mutex<()> = Mutex::const_new(());
        let _guard = tokio::select! {
            biased;
            _ = cancel.cancelled() => anyhow::bail!("Java installation cancelled while queued"),
            guard = LOCK.lock() => guard,
        };

        let catalog = self.managed_service.get_catalog(JavaVendor::Azul).await?;

        let build = pick_auto_install_build(&catalog, req).ok_or_else(|| {
            anyhow::anyhow!("unable to find automatically installable java version")
        })?;

        let id = self
            .managed_service
            .setup_managed(
                JavaVendor::Azul,
                build.id.clone(),
                self.app.clone(),
                progress,
            )
            .await?;

        let id_for_lookup = id.clone();
        let java = java_repo::get_java_by_id(&self.app.db, &id_for_lookup).await?;

        let java = match java {
            Some(java) => RealJavaChecker::get_bin_info(
                &RealJavaChecker::new(self.app.runtime_lifecycle.clone()),
                Path::new(&java.path),
                (&*java.r#type).try_into()?,
            )
            .await
            .map_err(|error| error.context("downloaded java was not runnable"))?,
            None => anyhow::bail!("downloaded java was not present in db"),
        };

        // The Java is installed and runs: a failed link must not fail the
        // launch or server waiting on it.
        if link_profiles && let Err(e) = link_requirement_profiles(&self.app.db, req, &id).await {
            tracing::warn!("could not link Java {id} to its profiles: {e:#}");
        }
        invalidate_java_lists(self.app);

        Ok(Some(java))
    }

    /// Find a Java appropriate for a server running the given Minecraft version.
    ///
    /// Resolves the version's java profile from the version manifest (matching
    /// the instance launch behavior), preferring the profile's linked Java, then
    /// any valid installed Java with a compatible version, then an automatic
    /// install. Falls back to the newest installed Java when the version's
    /// requirement cannot be determined.
    pub async fn find_java_for_server_version(
        self,
        game_version: &str,
        modloader_type: Option<&str>,
    ) -> anyhow::Result<PathBuf> {
        let Some(required_profile) = self
            .required_profile_for_server_version(game_version, modloader_type)
            .await
        else {
            tracing::warn!(
                "Could not determine the java requirement for Minecraft {game_version}, \
                 using the newest installed Java"
            );
            return self.find_best_java_for_server().await;
        };

        if let Some(java) = self
            .get_usable_java_for_profile_name(required_profile)
            .await?
        {
            return Ok(PathBuf::from(java.path));
        }

        // The profile has no usable linked Java — accept any valid installed
        // Java with a compatible version.
        let javas = self.get_available_javas().await?;
        for versions in javas.values() {
            for java in versions {
                if java.is_valid
                    && required_profile.is_java_version_compatible(&java.component.version)
                {
                    return Ok(PathBuf::from(&java.component.path));
                }
            }
        }

        let major = required_profile.major();
        match self
            .require_java_install(&requirement_for(required_profile), true, None)
            .await
        {
            Ok(Some(java)) => Ok(PathBuf::from(java.path)),
            Ok(None) => bail!(
                "Minecraft {game_version} servers need Java {major}, which could not be installed"
            ),
            Err(err) => Err(err.context(format!(
                "Minecraft {game_version} servers need Java {major}, which could not be installed"
            ))),
        }
    }

    /// Resolve the system java profile a server of the given Minecraft version
    /// requires, from the version manifest. Forge on 1.16.5 needs the patched
    /// legacy profile (Java 8 below update 312), same as the instance launch path.
    async fn required_profile_for_server_version(
        self,
        game_version: &str,
        modloader_type: Option<&str>,
    ) -> Option<SystemJavaProfileName> {
        if game_version == "1.16.5" && modloader_type == Some("forge") {
            return Some(SystemJavaProfileName::LegacyFixed1);
        }

        let manifest = match self.app.minecraft_manager().get_minecraft_manifest().await {
            Ok(manifest) => manifest,
            Err(e) => {
                tracing::warn!(
                    "Could not get the version manifest to determine the java requirement \
                     for Minecraft {game_version}: {e}"
                );
                return None;
            }
        };

        let java_profile = manifest
            .versions
            .iter()
            .find(|version| version.id == game_version)?
            .java_profile
            .clone()?;

        SystemJavaProfileName::try_from(java_profile).ok()
    }

    /// Find the best available Java for running a Minecraft server.
    /// Prefers the highest major version available. Returns the path to the java binary.
    pub async fn find_best_java_for_server(self) -> anyhow::Result<PathBuf> {
        let javas = self.get_available_javas().await?;

        // Get highest major version java that is valid
        let mut best: Option<(u8, PathBuf)> = None;
        for (major, versions) in &javas {
            for java in versions {
                if java.is_valid {
                    match &best {
                        Some((best_major, _)) if major > best_major => {
                            best = Some((*major, PathBuf::from(&java.component.path)));
                        }
                        None => {
                            best = Some((*major, PathBuf::from(&java.component.path)));
                        }
                        _ => {}
                    }
                }
            }
        }

        best.map(|(_, path)| path).ok_or_else(|| {
            anyhow::anyhow!("No Java installation found. Please install Java first.")
        })
    }
}

// Kept separate from manager construction so the database boundary can be
// exercised without starting discovery, Java processes or background services.
async fn create_custom_profile(
    db: &carbon_repos::db_exec::Db,
    profile_name: String,
    java_id: Option<String>,
) -> anyhow::Result<()> {
    // make sure profile doesn't start with system profile prefix
    if profile_name.starts_with(SYSTEM_JAVA_PROFILE_NAME_PREFIX) {
        anyhow::bail!(
            "Profile name cannot start with {}",
            SYSTEM_JAVA_PROFILE_NAME_PREFIX
        );
    }

    let java_id = java_id.ok_or_else(|| anyhow::anyhow!("java_id is required"))?;

    let name_for_lookup = profile_name.clone();
    let exists = java_repo::get_profile(db, &name_for_lookup).await?;

    if exists.is_some() {
        anyhow::bail!("Profile with name {} already exists", profile_name);
    }

    #[cfg(test)]
    PROFILE_LOOKUPS.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    // The unique name boundary and its valid Java assignment commit together.
    // The pre-read is only a friendly error; it cannot authorize an upsert.
    db.write(move |mut conn| {
        let tx = conn.transaction()?;
        if java_repo::insert_profile_conn(&tx, &profile_name, &java_id)? != 1 {
            return Err(carbon_repos::db_error::DbError::Conversion(
                "Selected Java no longer exists".into(),
            ));
        }
        tx.commit()?;
        Ok(())
    })
    .await?;

    Ok(())
}

#[cfg(test)]
static PROFILE_LOOKUPS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// GDLauncher links its own profiles, one per Minecraft Java version, so they
/// are never changed or deleted by hand.
fn refuse_system_profile(profile_name: &str) -> anyhow::Result<()> {
    if profile_name.starts_with(SYSTEM_JAVA_PROFILE_NAME_PREFIX) {
        bail!("GDLauncher picks this Java automatically, so it can't be changed by hand");
    }

    Ok(())
}

/// Call after anything that adds, removes or changes a Java.
pub(crate) fn invalidate_java_lists(app: &crate::App) {
    app.invalidate(GET_AVAILABLE_JAVAS, None);
    app.invalidate(GET_JAVA_PROFILES, None);
    app.invalidate(SYSTEM_JAVA_PROFILE_ASSIGNMENTS, None);
    app.invalidate(GET_MANAGED_JAVA_UPDATES, None);
}

/// The last catalogs fetched, however old, for what shows them rather than
/// installs from them: Settings › Java, the instance's Java dropdown and the
/// advisory checks never wait on the network. When one was never fetched or
/// is stale, they are fetched in the background, and what shows them follows.
pub(crate) fn shown_catalogs(app: &crate::App) -> HashMap<JavaVendor, ManagedJavaCatalog> {
    let service = &app.java_manager().managed_service;

    if let Some(claim) = service.claim_stale_refresh() {
        let app = app.clone();
        tokio::spawn(async move {
            let manager = app.java_manager();
            refresh_catalogs(&app, &manager.managed_service).await;
            drop(claim);
        });
    }

    service.cached_catalogs()
}

/// Every distribution's catalog that can be fetched, the fresh ones from
/// cache. After one is fetched, what shows the catalogs follows; from cache,
/// nothing changed.
pub(crate) async fn refresh_catalogs(
    app: &crate::App,
    service: &ManagedService,
) -> HashMap<JavaVendor, ManagedJavaCatalog> {
    let stale = service.stale_vendors();
    let catalogs = service.get_catalogs().await;

    if stale.iter().any(|vendor| catalogs.contains_key(vendor)) {
        // Updates, and with them the overview, mismatches and advisory
        // matches; instance details for Java 16, which the catalog may lack.
        app.invalidate(GET_MANAGED_JAVA_UPDATES, None);
        app.invalidate(INSTANCE_DETAILS, None);
    }

    catalogs
}

#[cfg(test)]
mod test {
    use crate::{
        domain::java::{JavaArch, JavaOs, JavaVendor, SystemJavaProfileName},
        setup_managers_for_test,
    };

    #[tokio::test]
    async fn startup_leaves_the_profiles_alone_until_the_defaults_migration_ran() {
        use super::{
            JavaManager,
            discovery::MockDiscovery,
            java_checker::MockJavaChecker,
            test_support::{add_managed, add_other, linked},
        };

        let app = setup_managers_for_test().await;
        add_managed(&app.app, "m21", "zulu21.0.5-a", "21.0.5", 10).await;
        // Picked by hand while Automatic was off.
        add_other(&app.app, "c21", "custom", "/opt/jdk-21/bin/java", "21.0.3").await;
        carbon_repos::repos::java::set_profile_java(
            &app.db,
            &SystemJavaProfileName::Delta.to_string(),
            Some("c21"),
        )
        .await
        .unwrap();

        JavaManager::scan_and_sync(&app.db, &MockDiscovery, &MockJavaChecker, false)
            .await
            .unwrap();
        assert_eq!(
            linked(&app.app, SystemJavaProfileName::Delta)
                .await
                .as_deref(),
            Some("c21")
        );

        JavaManager::scan_and_sync(&app.db, &MockDiscovery, &MockJavaChecker, true)
            .await
            .unwrap();
        assert_eq!(
            linked(&app.app, SystemJavaProfileName::Delta)
                .await
                .as_deref(),
            Some("m21")
        );
    }

    #[tokio::test]
    async fn a_server_without_its_java_tries_to_install_it_and_says_which_java() {
        let app = super::test_support::test_app(vec![]).await;
        // Forced on now: an old "off" no longer stops the download.
        app.db
            .write(|conn| {
                let patch = carbon_repos::repos::app_configuration::AppConfigurationPatch {
                    auto_manage_java_system_profiles: Some(false),
                    ..Default::default()
                };
                Ok(patch.build().map(|q| q.execute(&conn)).transpose()?)
            })
            .await
            .unwrap();

        let error = app
            .java_manager()
            .find_java_for_server_version("1.21.4", None)
            .await
            .unwrap_err();
        let message = format!("{error:#}");

        assert!(message.contains("need Java 21"), "{message}");
        // The empty catalog has nothing to install: it tried.
        assert!(message.contains("installable"), "{message}");
        assert!(!message.to_lowercase().contains("delta"), "{message}");
    }

    #[tokio::test]
    async fn invalidate_java_lists_emits_list_keys() {
        let mut app = setup_managers_for_test().await;

        super::invalidate_java_lists(&app.app);

        let mut keys = Vec::new();
        while let Ok(event) = app.invalidation_recv.try_recv() {
            keys.push(event.key);
        }

        assert!(keys.contains(&"java.getAvailableJavas"), "{keys:?}");
        assert!(keys.contains(&"java.getJavaProfiles"), "{keys:?}");
    }

    #[tokio::test]
    async fn what_shows_the_catalogs_follows_a_refresh_only_when_one_was_fetched() {
        use super::{
            managed::{ManagedService, azul_zulu::AzulZulu, mojang::MojangRuntime},
            refresh_catalogs,
        };

        let mut app = setup_managers_for_test().await;
        let mut sent = |app: &mut crate::TestEnv| {
            let mut keys = Vec::new();
            while let Ok(event) = app.invalidation_recv.try_recv() {
                keys.push(event.key);
            }
            keys
        };
        sent(&mut app);

        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", mockito::Matcher::Any)
            .with_status(500)
            .create_async()
            .await;
        let failing = ManagedService::with_runtimes(
            AzulZulu::with_base_url(format!("{}/", server.url())),
            MojangRuntime::with_base_url(format!("{}/", server.url())),
        );
        assert!(refresh_catalogs(&app.app, &failing).await.is_empty());
        assert!(sent(&mut app).is_empty());

        // Both fresh: nothing to fetch, so nothing changed.
        let fresh = ManagedService::new();
        fresh.cache_azul_catalog_for_test(Default::default()).await;
        fresh
            .cache_mojang_catalog_for_test(Default::default())
            .await;
        assert!(!refresh_catalogs(&app.app, &fresh).await.is_empty());
        assert!(sent(&mut app).is_empty());

        // Azul answers; Mojang still fails.
        let mut azul = mockito::Server::new_async().await;
        azul.mock("GET", mockito::Matcher::Any)
            .with_status(200)
            .with_header("X-Pagination", r#"{"total":0,"total_pages":0}"#)
            .with_body("[]")
            .create_async()
            .await;
        let fetched = ManagedService::with_runtimes(
            AzulZulu::with_base_url(format!("{}/", azul.url())),
            MojangRuntime::with_base_url(format!("{}/", server.url())),
        );
        assert!(!refresh_catalogs(&app.app, &fetched).await.is_empty());
        let keys = sent(&mut app);
        assert!(keys.contains(&"java.getManagedJavaUpdates"), "{keys:?}");
        assert!(keys.contains(&"java.getJavaOverview"), "{keys:?}");
        assert!(keys.contains(&"instance.getInstanceDetails"), "{keys:?}");
    }

    #[tokio::test]
    #[ignore]
    async fn test_require_java_install() {
        let app = setup_managers_for_test().await;

        let java_manager = app.java_manager();

        // Should update gamma, beta and gamma_snapshot
        let _ = java_manager
            .require_java_install(
                &super::resolve::requirement_for(SystemJavaProfileName::Gamma),
                true,
                None,
            )
            .await
            .unwrap()
            .unwrap();

        let profiles_in_db = carbon_repos::repos::java::get_all_profiles(&app.db)
            .await
            .unwrap()
            .into_iter()
            .filter(|p| p.is_system_profile)
            .collect::<Vec<_>>();

        assert_eq!(
            profiles_in_db
                .iter()
                .filter(|p| p.java_id.is_some())
                .count(),
            3
        );
    }

    #[tokio::test]
    async fn test_managed_service() {
        let app = setup_managers_for_test().await;

        let catalog = app
            .java_manager()
            .managed_service
            .get_catalog(JavaVendor::Azul)
            .await
            .unwrap();

        app.java_manager()
            .managed_service
            .setup_managed(
                JavaVendor::Azul,
                catalog.latest_for_major(17).unwrap().id.clone(),
                app.app.clone(),
                None,
            )
            .await
            .unwrap();
        let count = carbon_repos::repos::java::count_java(&app.db)
            .await
            .unwrap();
        assert_eq!(count, 1);

        let from_db = carbon_repos::repos::java::get_all_java(&app.db)
            .await
            .unwrap()
            .into_iter()
            .next()
            .unwrap();

        assert!(std::path::Path::new(&from_db.path).exists());

        let result_first_delete = app
            .java_manager()
            .delete_java_version(from_db.id.clone())
            .await;

        assert!(result_first_delete.is_ok());

        app.db
            .write(|conn| {
                let patch = carbon_repos::repos::app_configuration::AppConfigurationPatch {
                    auto_manage_java_system_profiles: Some(false),
                    ..Default::default()
                };
                Ok(patch.build().map(|q| q.execute(&conn)).transpose()?)
            })
            .await
            .unwrap();

        let count = carbon_repos::repos::java::count_java(&app.db)
            .await
            .unwrap();
        assert_eq!(count, 0);

        assert!(!std::path::Path::new(&from_db.path).exists());

        let managed_javas_root = app
            .settings_manager()
            .runtime_path
            .get_managed_javas()
            .to_path();

        let children = std::fs::read_dir(managed_javas_root).unwrap();

        assert_eq!(children.count(), 0);
    }
}

#[cfg(test)]
mod profile_tests {
    use super::*;
    #[tokio::test]
    async fn concurrent_profile_creation_keeps_winners_assignment() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("profile.db");
        {
            let mut c = rusqlite::Connection::open(&path).unwrap();
            carbon_repos::get_migrations().0.to_latest(&mut c).unwrap();
            c.execute_batch("INSERT INTO Java(id,path,major,fullVersion,type,os,arch,vendor,isValid) VALUES ('jdk8','fixture8',8,'8','local','linux','x64','unknown',1),('jdk21','fixture21',21,'21','local','linux','x64','unknown',1)").unwrap();
        }
        let db = carbon_repos::db_exec::Db::open(&path, 2, true).unwrap();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let held = db.write(move |_c| {
            let _ = entered_tx.send(());
            release_rx.recv().unwrap();
            Ok(())
        });
        let work = async {
            entered_rx.await.unwrap();
            let a = create_custom_profile(&db, "new".into(), Some("jdk8".into()));
            let b = create_custom_profile(&db, "new".into(), Some("jdk21".into()));
            let release = async {
                while PROFILE_LOOKUPS.load(std::sync::atomic::Ordering::SeqCst) < 2 {
                    tokio::task::yield_now().await;
                }
                release_tx.send(()).unwrap();
            };
            let (a, b, ()) = tokio::join!(a, b, release);
            assert_ne!(
                a.is_ok(),
                b.is_ok(),
                "exactly one create succeeds: {a:?} / {b:?}"
            );
            let winner = if a.is_ok() { "jdk8" } else { "jdk21" };
            assert_eq!(
                java_repo::get_profile(&db, "new")
                    .await
                    .unwrap()
                    .unwrap()
                    .java_id
                    .as_deref(),
                Some(winner)
            );
            assert!(
                create_custom_profile(&db, "new".into(), Some("jdk8".into()))
                    .await
                    .is_err()
            );
            assert!(
                create_custom_profile(&db, "missing".into(), None)
                    .await
                    .is_err()
            );
            assert!(
                create_custom_profile(
                    &db,
                    format!("{}test", SYSTEM_JAVA_PROFILE_NAME_PREFIX),
                    Some("jdk8".into())
                )
                .await
                .is_err()
            );
            assert!(
                create_custom_profile(&db, "bad-parent".into(), Some("absent".into()))
                    .await
                    .is_err()
            );
            assert!(
                java_repo::get_profile(&db, "bad-parent")
                    .await
                    .unwrap()
                    .is_none()
            );
        };
        let (held, ()) = tokio::join!(held, work);
        held.unwrap();
        let reopened = rusqlite::Connection::open(path).unwrap();
        assert_eq!(
            reopened
                .query_row(
                    "SELECT count(*) FROM JavaProfile WHERE name='new'",
                    [],
                    |r| r.get::<_, i32>(0)
                )
                .unwrap(),
            1
        );
    }
}
