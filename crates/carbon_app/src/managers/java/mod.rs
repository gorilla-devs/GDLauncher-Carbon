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
use carbon_repos::{DbPool, models, queries};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};
use strum::IntoEnumIterator;
use tokio::sync::{Mutex, watch};
use tracing::{debug, error, trace};

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

    pub async fn ensure_profiles_in_db(db_pool: &DbPool) -> anyhow::Result<()> {
        debug!("Ensuring system java profiles are in db");

        let profiles_to_check: Vec<_> = SystemJavaProfileName::iter().collect();
        let pool = db_pool.clone();

        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;

            for profile in profiles_to_check {
                let profile_name = profile.to_string();

                // Check if profile exists
                let exists: Option<models::JavaProfile> = conn
                    .query_row(
                        queries::java::FindJavaProfileByName::SQL,
                        rusqlite::params![&profile_name],
                        |row| models::JavaProfile::from_row(row),
                    )
                    .ok();

                if let Some(existing) = exists {
                    if !existing.is_system_profile {
                        conn.execute(
                            "UPDATE JavaProfile SET isSystemProfile = 1 WHERE name = ?1",
                            rusqlite::params![&profile_name],
                        )?;
                    }
                } else {
                    match conn.execute(
                        queries::java::CreateJavaProfile::SQL,
                        rusqlite::params![&profile_name, true, Option::<String>::None],
                    ) {
                        Err(error) => {
                            error!("Error creating profile {profile:?}: {error}");
                            return Err(error.into());
                        }
                        Ok(_) => {
                            trace!("Profile {profile:?} created");
                        }
                    }
                }
            }

            Ok::<_, anyhow::Error>(())
        })
        .await??;

        Ok(())
    }

    pub async fn scan_and_sync<T, G>(
        auto_manage_java_system_profiles: bool,
        db_pool: &DbPool,
        discovery: &T,
        java_checker: &G,
    ) -> anyhow::Result<()>
    where
        T: Discovery,
        G: JavaChecker,
    {
        scan_and_sync::scan_and_sync_local(db_pool, discovery, java_checker).await?;
        scan_and_sync::scan_and_sync_custom(db_pool, java_checker).await?;
        scan_and_sync::scan_and_sync_managed(db_pool, discovery, java_checker).await?;

        if auto_manage_java_system_profiles {
            scan_and_sync::sync_system_java_profiles(db_pool).await?;
        }

        Ok(())
    }
}

impl ManagerRef<'_, JavaManager> {
    pub async fn get_available_javas(&self) -> anyhow::Result<HashMap<u8, Vec<Java>>> {
        let pool = self.app.db_pool.clone();

        let all_javas = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(queries::java::ListJavas::SQL)?;
            let javas = stmt
                .query_map([], |row| models::Java::from_row(row))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, anyhow::Error>(javas)
        })
        .await??;

        let mut result = HashMap::new();

        for java in all_javas {
            let major_version = java.major as u8;
            let javas = result.entry(major_version).or_insert_with(Vec::new);
            javas.push(Java::try_from(java)?);
        }

        Ok(result)
    }

    pub async fn get_java_profiles(&self) -> anyhow::Result<Vec<JavaProfile>> {
        let pool = self.app.db_pool.clone();

        let all_profiles = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let mut stmt = conn.prepare(queries::java::ListJavaProfiles::SQL)?;
            let profiles = stmt
                .query_map([], |row| models::JavaProfile::from_row(row))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok::<_, anyhow::Error>(profiles)
        })
        .await??;

        let all_profiles = all_profiles
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

        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            conn.execute(
                queries::java::UpdateJavaProfileJavaId::SQL,
                rusqlite::params![&profile_name, &java_id],
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

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

        let pool = self.app.db_pool.clone();
        let profile_name_clone = profile_name.clone();

        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;

            // Check if profile exists
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM JavaProfile WHERE name = ?1",
                    rusqlite::params![&profile_name_clone],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if exists {
                anyhow::bail!("Profile with name {} already exists", profile_name_clone);
            }

            conn.execute(
                queries::java::CreateJavaProfile::SQL,
                rusqlite::params![&profile_name_clone, false, Some(&java_id)],
            )?;

            Ok::<_, anyhow::Error>(())
        })
        .await??;

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

        let pool = self.app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            conn.execute(
                queries::java::DeleteJavaProfile::SQL,
                rusqlite::params![&profile_name],
            )?;
            Ok::<_, anyhow::Error>(())
        })
        .await??;

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

        let pool = self.app.db_pool.clone();
        let path_clone = path.clone();

        tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;

            // Check if java with this path already exists
            let exists: bool = conn
                .query_row(
                    "SELECT COUNT(*) > 0 FROM Java WHERE path = ?1",
                    rusqlite::params![&path_clone],
                    |row| row.get(0),
                )
                .unwrap_or(false);

            if exists {
                anyhow::bail!("Java with path {} already exists", path_clone);
            }

            let id = uuid::Uuid::new_v4().to_string();
            conn.execute(
                queries::java::CreateJava::SQL,
                rusqlite::params![
                    &id,
                    &java.path,
                    java.version.major as i32,
                    &java.version.to_string(),
                    &java._type.to_string(),
                    &java.os.to_string(),
                    &java.arch.to_string(),
                    &java.vendor,
                    true
                ],
            )?;

            Ok::<_, anyhow::Error>(())
        })
        .await??;

        self.app.invalidate(GET_AVAILABLE_JAVAS, None);

        Ok(())
    }

    pub async fn delete_java_version(&self, java_id: String) -> anyhow::Result<()> {
        let pool = self.app.db_pool.clone();
        let java_id_clone = java_id.clone();

        let java_from_db = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let java: models::Java = conn.query_row(
                queries::java::FindJavaById::SQL,
                rusqlite::params![&java_id_clone],
                |row| models::Java::from_row(row),
            )?;
            Ok::<_, anyhow::Error>(java)
        })
        .await??;

        let java_component_type = JavaComponentType::try_from(&*java_from_db.java_type)?;

        match java_component_type {
            JavaComponentType::Custom => {
                let pool = self.app.db_pool.clone();
                let java_id_clone = java_id.clone();
                tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    conn.execute(
                        queries::java::DeleteJava::SQL,
                        rusqlite::params![&java_id_clone],
                    )?;
                    Ok::<_, anyhow::Error>(())
                })
                .await??;
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

                let pool = self.app.db_pool.clone();
                let java_id_clone = java_id.clone();
                tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    conn.execute(
                        queries::java::DeleteJava::SQL,
                        rusqlite::params![&java_id_clone],
                    )?;
                    Ok::<_, anyhow::Error>(())
                })
                .await??;
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
        let pool = self.app.db_pool.clone();
        let target_profile_name = target_profile.to_string();

        // Find the profile
        let profile = tokio::task::spawn_blocking({
            let pool = pool.clone();
            let target_profile_name = target_profile_name.clone();
            move || {
                let conn = pool.get()?;
                let mut stmt = conn.prepare(queries::java::ListSystemJavaProfiles::SQL)?;
                let profiles: Vec<models::JavaProfile> = stmt
                    .query_map([], |row| models::JavaProfile::from_row(row))?
                    .collect::<Result<Vec<_>, _>>()?;
                profiles
                    .into_iter()
                    .find(|p| p.name == target_profile_name)
                    .ok_or_else(|| anyhow::anyhow!("Profile not found"))
            }
        })
        .await??;

        // Find associated java if any
        let java = match profile.java_id {
            Some(java_id) => {
                let pool = pool.clone();
                tokio::task::spawn_blocking(move || {
                    let conn = pool.get()?;
                    let java: Option<models::Java> = conn
                        .query_row(
                            queries::java::FindJavaById::SQL,
                            rusqlite::params![&java_id],
                            |row| models::Java::from_row(row),
                        )
                        .ok();
                    Ok::<_, anyhow::Error>(java)
                })
                .await??
            }
            None => None,
        };

        let java = match java {
            Some(java) => {
                let bin_result = RealJavaChecker::get_bin_info(
                    &RealJavaChecker,
                    Path::new(&java.path),
                    (&*java.java_type).try_into()?,
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

                        // Update all profiles using this java to disconnect
                        let pool = self.app.db_pool.clone();
                        let java_id = java.id.clone();
                        tokio::task::spawn_blocking(move || {
                            let conn = pool.get()?;

                            // Find all profiles using this java
                            let mut stmt =
                                conn.prepare("SELECT * FROM JavaProfile WHERE javaId = ?1")?;
                            let profiles: Vec<models::JavaProfile> = stmt
                                .query_map(rusqlite::params![&java_id], |row| {
                                    models::JavaProfile::from_row(row)
                                })?
                                .collect::<Result<Vec<_>, _>>()?;

                            // Disconnect them
                            for profile in profiles {
                                conn.execute(
                                    queries::java::UpdateJavaProfileJavaId::SQL,
                                    rusqlite::params![&profile.name, Option::<String>::None],
                                )?;
                            }

                            // Mark java as invalid
                            conn.execute(
                                queries::java::UpdateJavaValid::SQL,
                                rusqlite::params![&java_id, false],
                            )?;

                            Ok::<_, anyhow::Error>(())
                        })
                        .await??;

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

        let pool = self.app.db_pool.clone();
        let id_clone = id.clone();

        let java_from_db = tokio::task::spawn_blocking(move || {
            let conn = pool.get()?;
            let java: Option<models::Java> = conn
                .query_row(
                    queries::java::FindJavaById::SQL,
                    rusqlite::params![&id_clone],
                    |row| models::Java::from_row(row),
                )
                .ok();
            Ok::<_, anyhow::Error>(java)
        })
        .await??;

        let java = match java_from_db {
            Some(java) => RealJavaChecker::get_bin_info(
                &RealJavaChecker,
                Path::new(&java.path),
                (&*java.java_type).try_into()?,
            )
            .await
            .map_err(|_| anyhow::anyhow!("downloaded java was not runnable"))?,
            None => anyhow::bail!("downloaded java was not present in db"),
        };

        if update_target_profile {
            let pool = self.app.db_pool.clone();
            let target_profile_name = target_profile.to_string();
            let id_clone = id.clone();
            let java_version = java.version.clone();

            tokio::task::spawn_blocking(move || {
                let conn = pool.get()?;

                // Update target profile
                conn.execute(
                    queries::java::UpdateJavaProfileJavaId::SQL,
                    rusqlite::params![&target_profile_name, Some(&id_clone)],
                )?;

                // Get all system profiles
                let mut stmt = conn.prepare(queries::java::ListSystemJavaProfiles::SQL)?;
                let system_profiles: Vec<models::JavaProfile> = stmt
                    .query_map([], |row| models::JavaProfile::from_row(row))?
                    .collect::<Result<Vec<_>, _>>()?;

                for system_profile in system_profiles {
                    let system_profile_name_result =
                        SystemJavaProfileName::try_from(&*system_profile.name);
                    let Ok(system_profile_name) = system_profile_name_result else {
                        continue;
                    };

                    if system_profile_name == target_profile
                        || !system_profile_name.is_java_version_compatible(&java_version)
                        || system_profile.java_id.is_some()
                    {
                        continue;
                    }

                    conn.execute(
                        queries::java::UpdateJavaProfileJavaId::SQL,
                        rusqlite::params![&system_profile.name, Some(&id_clone)],
                    )?;
                }

                Ok::<_, anyhow::Error>(())
            })
            .await??;

            self.app.invalidate(GET_JAVA_PROFILES, None);
        }

        Ok(Some(java))
    }
}

#[cfg(test)]
mod test {
    use super::*;
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

        let pool = app.db_pool.clone();
        let profiles_in_db = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            let mut stmt = conn
                .prepare(queries::java::ListSystemJavaProfiles::SQL)
                .unwrap();
            let profiles: Vec<models::JavaProfile> = stmt
                .query_map([], |row| models::JavaProfile::from_row(row))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            profiles
        })
        .await
        .unwrap();

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

        let pool = app.db_pool.clone();
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            conn.query_row(queries::java::CountJavas::SQL, [], |row| row.get(0))
                .unwrap()
        })
        .await
        .unwrap();
        assert_eq!(count, 1);

        let pool = app.db_pool.clone();
        let from_db: models::Java = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            let mut stmt = conn.prepare(queries::java::ListJavas::SQL).unwrap();
            let java = stmt
                .query_row([], |row| models::Java::from_row(row))
                .unwrap();
            java
        })
        .await
        .unwrap();

        assert!(std::path::Path::new(&from_db.path).exists());

        let result_first_delete = app
            .java_manager()
            .delete_java_version(from_db.id.clone())
            .await;

        assert!(result_first_delete.is_ok());

        let pool = app.db_pool.clone();
        tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            conn.execute(
                "UPDATE AppConfiguration SET autoManageJavaSystemProfiles = 0 WHERE id = 0",
                [],
            )
            .unwrap();
        })
        .await
        .unwrap();

        let pool = app.db_pool.clone();
        let count: i64 = tokio::task::spawn_blocking(move || {
            let conn = pool.get().unwrap();
            conn.query_row(queries::java::CountJavas::SQL, [], |row| row.get(0))
                .unwrap()
        })
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
