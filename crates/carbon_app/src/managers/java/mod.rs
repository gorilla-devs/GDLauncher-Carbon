use self::{
    discovery::Discovery,
    java_checker::JavaChecker,
    managed::{ManagedService, Step},
};
use super::ManagerRef;
use crate::{
    api::keys::java::{GET_AVAILABLE_JAVAS, GET_JAVA_PROFILES},
    domain::{
        instance::info::StandardVersion,
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

mod constants;
pub mod discovery;
pub mod java_checker;
pub mod managed;
mod parser;
pub mod scan_and_sync;
pub mod utils;

pub(crate) struct JavaManager {
    pub managed_service: ManagedService,
}

impl JavaManager {
    pub fn new() -> Self {
        Self {
            managed_service: ManagedService::new(),
        }
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

    pub async fn scan_and_sync<T, G>(
        auto_manage_java_system_profiles: bool,
        db: &Db,
        discovery: &T,
        java_checker: &G,
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

        if auto_manage_java_system_profiles {
            let t = std::time::Instant::now();
            scan_and_sync::sync_system_java_profiles(db).await?;
            tracing::debug!(
                "[startup-timing] sync_system_java_profiles completed in {:.2}s",
                t.elapsed().as_secs_f64()
            );
        }

        Ok(())
    }
}

impl ManagerRef<'_, JavaManager> {
    pub async fn get_available_javas(&self) -> anyhow::Result<HashMap<u8, Vec<Java>>> {
        let all_javas = java_repo::get_all_java(&self.app.db)
            .await?;

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

        let java = RealJavaChecker::get_bin_info(&RealJavaChecker, p, JavaComponentType::Custom)
            .await
            .is_ok();

        Ok(java)
    }

    pub async fn update_java_profile(
        &self,
        profile_name: String,
        java_id: Option<String>,
    ) -> anyhow::Result<()> {
        let auto_manage_java_system_profiles = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .auto_manage_java_system_profiles;

        if auto_manage_java_system_profiles
            && profile_name.starts_with(SYSTEM_JAVA_PROFILE_NAME_PREFIX)
        {
            anyhow::bail!("Auto manage java is enabled");
        }

        if let Some(java_id) = java_id {
            java_repo::set_profile_java(&self.app.db, &profile_name, Some(&java_id)).await?;
        } else {
            java_repo::set_profile_java(&self.app.db, &profile_name, None)
                .await?;
        }

        self.app.invalidate(GET_JAVA_PROFILES, None);

        Ok(())
    }

    pub async fn create_java_profile(
        &self,
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
        let exists = java_repo::get_profile(&self.app.db, &name_for_lookup)
            .await?;

        if exists.is_some() {
            anyhow::bail!("Profile with name {} already exists", profile_name);
        }

        // Two statements (create the profile, then link its java) run in one
        // writer dispatch and ONE transaction — all-or-nothing: the profile and
        // its java link land together or not at all, and readers never observe
        // an intermediate state. `_conn` forms on the tx guard.
        self.app
            .db
            .write(move |mut conn| {
                let tx = conn.transaction()?;
                java_repo::upsert_profile_conn(&tx, &profile_name, false)?;
                java_repo::set_profile_java_conn(&tx, &profile_name, Some(&java_id))?;
                tx.commit()?;
                Ok(())
            })
            .await?;

        self.app.invalidate(GET_JAVA_PROFILES, None);

        Ok(())
    }

    pub async fn delete_java_profile(&self, profile_name: String) -> anyhow::Result<()> {
        let auto_manage_java_system_profiles = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .auto_manage_java_system_profiles;

        if auto_manage_java_system_profiles
            && profile_name.starts_with(SYSTEM_JAVA_PROFILE_NAME_PREFIX)
        {
            anyhow::bail!("Auto manage java is enabled");
        }

        java_repo::delete_profile(&self.app.db, &profile_name)
            .await?;

        self.app.invalidate(GET_JAVA_PROFILES, None);

        Ok(())
    }

    pub async fn create_custom_java_version(&self, path: String) -> anyhow::Result<()> {
        let java = RealJavaChecker::get_bin_info(
            &RealJavaChecker,
            Path::new(&path),
            JavaComponentType::Custom,
        )
        .await?;

        let path_for_lookup = path.clone();
        let exists = java_repo::get_java_by_path(&self.app.db, &path_for_lookup)
            .await?;

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

        match java_component_type {
            JavaComponentType::Custom => {
                let id = java_id.clone();
                java_repo::delete_java(&self.app.db, &id)
                    .await?;
            }
            JavaComponentType::Managed => {
                let root_managed_path = self
                    .app
                    .settings_manager()
                    .runtime_path
                    .get_managed_javas()
                    .to_path();
                let java_bin_path = PathBuf::from(java_from_db.path);

                let managed_java_dir_name = java_bin_path
                    .strip_prefix(&root_managed_path)?
                    .components()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("Could not strip prefix"))?;

                let managed_java_dir = root_managed_path.join(managed_java_dir_name);

                if managed_java_dir.exists() {
                    std::fs::remove_dir_all(managed_java_dir)?;
                }

                let id = java_id.clone();
                java_repo::delete_java(&self.app.db, &id)
                    .await?;
            }
            JavaComponentType::Local => {
                anyhow::bail!("Java with id {} is local. Cannot delete.", java_id.clone());
            }
        }

        self.app.invalidate(GET_JAVA_PROFILES, None);
        self.app.invalidate(GET_AVAILABLE_JAVAS, None);

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_usable_java_for_profile_name(
        self,
        target_profile: SystemJavaProfileName,
    ) -> anyhow::Result<Option<JavaComponent>> {
        let target_name = target_profile.to_string();
        let profile = java_repo::get_profile(&self.app.db, &target_name)
            .await?
            .filter(|profile| profile.is_system_profile)
            .ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

        let java = match profile.java_id {
            Some(java_id) => {
                java_repo::get_java_by_id(&self.app.db, &java_id)
                    .await?
            }
            None => None,
        };

        let java = match java {
            Some(java) => {
                let bin_result = RealJavaChecker::get_bin_info(
                    &RealJavaChecker,
                    Path::new(&java.path),
                    (&*java.r#type).try_into()?,
                )
                .await;

                match bin_result {
                    Ok(bin_info) => Some(bin_info),
                    Err(err) => {
                        tracing::warn!(
                            "Java {} is not usable: {}. Cleaning it up from db",
                            java.id,
                            err
                        );

                        let java_id = java.id.clone();
                        let id_for_read = java_id.clone();
                        let names_to_disconnect: Vec<String> = java_repo::get_all_profiles(&self.app.db)
                            .await?
                            .into_iter()
                            .filter(|profile| profile.java_id.as_deref() == Some(id_for_read.as_str()))
                            .map(|profile| profile.name)
                            .collect();

                        // Interleaved app logic: unlink the computed set of
                        // profiles then flip the java's validity. Runs in one
                        // writer dispatch and ONE transaction — all-or-nothing:
                        // a failure rolls the whole group back and readers never
                        // observe an intermediate state. `_conn` forms on the tx guard.
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

    /// Will return Some(path) if configured to automatically install.
    /// Will return None if user intervention is required.
    pub async fn require_java_install(
        self,
        target_profile: SystemJavaProfileName,
        update_target_profile: bool,
        progress: Option<watch::Sender<Step>>,
    ) -> anyhow::Result<Option<JavaComponent>> {
        static LOCK: Mutex<()> = Mutex::const_new(());
        let _guard = LOCK.lock().await;

        let versions = self
            .app
            .java_manager()
            .managed_service
            .get_versions_for_vendor(JavaVendor::Azul)
            .await?;

        let current_os = JavaOs::get_current_os()?;
        let current_arch = JavaArch::get_current_arch()?;

        let id = self
            .managed_service
            .setup_managed(
                current_os,
                current_arch,
                JavaVendor::Azul,
                versions
                    .get(&current_os)
                    .and_then(|for_arch| for_arch.get(&current_arch))
                    .and_then(|versions| {
                        versions
                            .iter()
                            .find(|v| target_profile.is_java_version_compatible(&v.java_version))
                    })
                    .ok_or_else(|| {
                        anyhow::anyhow!("unable to find automatically installable java version")
                    })?
                    .id
                    .clone(),
                self.app.clone(),
                progress,
            )
            .await?;

        let id_for_lookup = id.clone();
        let java = java_repo::get_java_by_id(&self.app.db, &id_for_lookup)
            .await?;

        let java = match java {
            Some(java) => RealJavaChecker::get_bin_info(
                &RealJavaChecker,
                Path::new(&java.path),
                (&*java.r#type).try_into()?,
            )
            .await
            .map_err(|_| anyhow::anyhow!("downloaded java was not runnable"))?,
            None => anyhow::bail!("downloaded java was not present in db"),
        };

        if update_target_profile {
            let target_name = target_profile.to_string();
            java_repo::set_profile_java(&self.app.db, &target_name, Some(&id)).await?;

            let system_profiles_in_db = java_repo::get_all_profiles(&self.app.db)
                .await?
                .into_iter()
                .filter(|profile| profile.is_system_profile);

            let mut names_to_connect: Vec<String> = Vec::new();
            for system_profile in system_profiles_in_db {
                let system_profile_name = SystemJavaProfileName::try_from(&*system_profile.name)?;
                if system_profile_name == target_profile
                    || !system_profile_name.is_java_version_compatible(&java.version)
                    || system_profile.java_id.is_some()
                {
                    continue;
                }

                names_to_connect.push(system_profile.name);
            }

            if !names_to_connect.is_empty() {
                let id_for_connect = id.clone();
                // Interleaved app logic: link every compatible system profile to
                // the freshly installed java in one writer dispatch, so no other
                // write interleaves. Uses `_conn` forms inside the closure.
                self.app
                    .db
                    .write(move |conn| {
                        for name in names_to_connect {
                            java_repo::set_profile_java_conn(&conn, &name, Some(&id_for_connect))?;
                        }
                        Ok(())
                    })
                    .await?;
            }
            self.app.invalidate(GET_JAVA_PROFILES, None);
        }

        Ok(Some(java))
    }

    /// Find a Java appropriate for a server running the given Minecraft version.
    ///
    /// Resolves the version's java profile from the version manifest (matching
    /// the instance launch behavior), preferring the profile's linked Java, then
    /// any valid installed Java with a compatible version, then an automatic
    /// install when system java profiles are auto-managed. Falls back to the
    /// newest installed Java when the version's requirement cannot be determined.
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

        let auto_manage_java_system_profiles = self
            .app
            .settings_manager()
            .get_settings()
            .await?
            .auto_manage_java_system_profiles;

        if auto_manage_java_system_profiles {
            if let Some(java) = self
                .require_java_install(required_profile, true, None)
                .await?
            {
                return Ok(PathBuf::from(java.path));
            }
        }

        bail!(
            "Minecraft {game_version} servers require a {required_profile:?} Java which is not \
             installed. Install a compatible Java or enable automatic Java management."
        )
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

#[cfg(test)]
mod test {
    use crate::{
        domain::java::{JavaArch, JavaOs, JavaVendor, SystemJavaProfileName},
        setup_managers_for_test,
    };

    #[tokio::test]
    #[ignore]
    async fn test_require_java_install() {
        let app = setup_managers_for_test().await;

        let java_manager = app.java_manager();

        // Should update both gamma and beta
        let _ = java_manager
            .require_java_install(SystemJavaProfileName::Gamma, true, None)
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
            2
        );
    }

    #[tokio::test]
    async fn test_managed_service() {
        let app = setup_managers_for_test().await;

        let versions = app
            .java_manager()
            .managed_service
            .get_versions_for_vendor(JavaVendor::Azul)
            .await
            .unwrap();

        assert!(versions.contains_key(&JavaOs::Linux));
        assert!(versions.contains_key(&JavaOs::Windows));
        assert!(versions.contains_key(&JavaOs::MacOs));

        app.java_manager()
            .managed_service
            .setup_managed(
                JavaOs::get_current_os().unwrap(),
                JavaArch::get_current_arch().unwrap(),
                JavaVendor::Azul,
                versions
                    .get(&JavaOs::get_current_os().unwrap())
                    .unwrap()
                    .get(&JavaArch::get_current_arch().unwrap())
                    .unwrap()[0]
                    .id
                    .clone(),
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
