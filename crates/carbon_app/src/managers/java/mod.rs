use anyhow::bail;
use prisma_client_rust::{prisma_errors::query_engine::UniqueKeyViolation, QueryError};
use strum::IntoEnumIterator;
use tokio::sync::watch;
use tracing::{debug, error, trace};

use self::{
    discovery::Discovery,
    java_checker::JavaChecker,
    managed::{ManagedService, Step},
};

use super::ManagerRef;
use crate::{
    api::keys::java::{GET_AVAILABLE_JAVAS, GET_JAVA_PROFILES},
    db::PrismaClient,
    domain::{
        instance::info::StandardVersion,
        java::{
            Java, JavaArch, JavaComponent, JavaComponentType, JavaOs, JavaProfile, JavaVendor,
            SystemJavaProfileName, SYSTEM_JAVA_PROFILE_NAME_PREFIX,
        },
    },
    managers::java::java_checker::RealJavaChecker,
};
use std::{
    collections::HashMap,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::Arc,
};

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

    pub async fn ensure_profiles_in_db(db_client: &PrismaClient) -> anyhow::Result<()> {
        debug!("Ensuring system java profiles are in db");
        for profile in SystemJavaProfileName::iter() {
            let exists = db_client
                .java_profile()
                .find_unique(crate::db::java_profile::name::equals(profile.to_string()))
                .exec()
                .await?;

            if exists.is_some() {
                let exists = exists.unwrap();
                if !exists.is_system_profile {
                    db_client
                        .java_profile()
                        .update(
                            crate::db::java_profile::name::equals(profile.to_string()),
                            vec![crate::db::java_profile::is_system_profile::set(true)],
                        )
                        .exec()
                        .await?;
                }
            } else {
                let creation: Result<crate::db::java_profile::Data, QueryError> = db_client
                    .java_profile()
                    .create(
                        profile.to_string(),
                        vec![crate::db::java_profile::is_system_profile::set(true)],
                    )
                    .exec()
                    .await;

                match creation {
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

        Ok(())
    }

    pub async fn scan_and_sync<T, G>(
        auto_manage_java_system_profiles: bool,
        db: &Arc<PrismaClient>,
        discovery: &T,
        java_checker: &G,
    ) -> anyhow::Result<()>
    where
        T: Discovery,
        G: JavaChecker,
    {
        scan_and_sync::scan_and_sync_local(db, discovery, java_checker).await?;
        scan_and_sync::scan_and_sync_custom(db, java_checker).await?;
        scan_and_sync::scan_and_sync_managed(db, discovery, java_checker).await?;

        if auto_manage_java_system_profiles {
            scan_and_sync::sync_system_java_profiles(db).await?;
        }

        Ok(())
    }
}

impl ManagerRef<'_, JavaManager> {
    pub async fn get_available_javas(&self) -> anyhow::Result<HashMap<u8, Vec<Java>>> {
        let db = &self.app.prisma_client;
        let all_javas = db.java().find_many(vec![]).exec().await?;

        let mut result = HashMap::new();

        for java in all_javas {
            let major_version = java.major as u8;
            let javas = result.entry(major_version).or_insert_with(Vec::new);
            javas.push(Java::try_from(java)?);
        }

        Ok(result)
    }

    pub async fn get_java_profiles(&self) -> anyhow::Result<Vec<JavaProfile>> {
        let db = &self.app.prisma_client;
        let all_profiles = db
            .java_profile()
            .find_many(vec![])
            .exec()
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
            self.app
                .prisma_client
                .java_profile()
                .update(
                    crate::db::java_profile::name::equals(profile_name.to_string()),
                    vec![crate::db::java_profile::java::connect(
                        crate::db::java::id::equals(java_id),
                    )],
                )
                .exec()
                .await?;
        } else {
            self.app
                .prisma_client
                .java_profile()
                .update(
                    crate::db::java_profile::name::equals(profile_name.to_string()),
                    vec![crate::db::java_profile::java::disconnect()],
                )
                .exec()
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

        let exists = self
            .app
            .prisma_client
            .java_profile()
            .find_unique(crate::db::java_profile::name::equals(profile_name.clone()))
            .exec()
            .await?;

        if exists.is_some() {
            anyhow::bail!("Profile with name {} already exists", profile_name);
        }

        self.app
            .prisma_client
            .java_profile()
            .create(
                profile_name,
                vec![crate::db::java_profile::java::connect(
                    crate::db::java::id::equals(java_id),
                )],
            )
            .exec()
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

        self.app
            .prisma_client
            .java_profile()
            .delete(crate::db::java_profile::name::equals(profile_name))
            .exec()
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

        let exists = self
            .app
            .prisma_client
            .java()
            .find_unique(crate::db::java::path::equals(path.clone()))
            .exec()
            .await?;

        if exists.is_some() {
            anyhow::bail!("Java with path {} already exists", path);
        }

        self.app
            .prisma_client
            .java()
            .create(
                java.path,
                java.version.major as i32,
                java.version.to_string(),
                java._type.to_string(),
                java.os.to_string(),
                java.arch.to_string(),
                java.vendor,
                vec![],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_AVAILABLE_JAVAS, None);

        Ok(())
    }

    pub async fn delete_java_version(&self, java_id: String) -> anyhow::Result<()> {
        let java_from_db = self
            .app
            .prisma_client
            .java()
            .find_unique(crate::db::java::id::equals(java_id.clone()))
            .exec()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Java with id {} not found", java_id.clone()))?;

        let java_component_type = JavaComponentType::try_from(&*java_from_db.r#type)?;

        match java_component_type {
            JavaComponentType::Custom => {
                self.app
                    .prisma_client
                    .java()
                    .delete(crate::db::java::id::equals(java_id))
                    .exec()
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

                self.app
                    .prisma_client
                    .java()
                    .delete(crate::db::java::id::equals(java_id))
                    .exec()
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
        use crate::db::{java, java_profile};

        let profile = self
            .app
            .prisma_client
            .java_profile()
            .find_many(vec![crate::db::java_profile::is_system_profile::equals(
                true,
            )])
            .exec()
            .await?
            .into_iter()
            .find(|profile| profile.name == target_profile.to_string())
            .ok_or_else(|| anyhow::anyhow!("Profile not found"))?;

        let java = match profile.java_id {
            Some(java_id) => {
                self.app
                    .prisma_client
                    .java()
                    .find_unique(java::id::equals(java_id))
                    .exec()
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

                        let all_profiles_using_this_java = self
                            .app
                            .prisma_client
                            .java_profile()
                            .find_many(vec![java_profile::java_id::equals(Some(java.id.clone()))])
                            .exec()
                            .await?;

                        for profile in all_profiles_using_this_java {
                            self.app
                                .prisma_client
                                .java_profile()
                                .update(
                                    java_profile::name::equals(profile.name.to_string()),
                                    vec![java_profile::java::disconnect()],
                                )
                                .exec()
                                .await?;
                        }

                        self.app
                            .prisma_client
                            .java()
                            .update(
                                java::id::equals(java.id.clone()),
                                vec![java::is_valid::set(false)],
                            )
                            .exec()
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
        use crate::db::java::UniqueWhereParam;

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

        let java = self
            .app
            .prisma_client
            .java()
            .find_unique(crate::db::java::id::equals(id.clone()))
            .exec()
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
            self.app
                .prisma_client
                .java_profile()
                .update(
                    crate::db::java_profile::name::equals(target_profile.to_string()),
                    vec![crate::db::java_profile::java::connect(
                        crate::db::java::id::equals(id.clone()),
                    )],
                )
                .exec()
                .await?;

            let system_profiles_in_db = self
                .app
                .prisma_client
                .java_profile()
                .find_many(vec![crate::db::java_profile::is_system_profile::equals(
                    true,
                )])
                .exec()
                .await?;

            for system_profile in system_profiles_in_db {
                let system_profile_name = SystemJavaProfileName::try_from(&*system_profile.name)?;
                if system_profile_name == target_profile
                    || !system_profile_name.is_java_version_compatible(&java.version)
                    || system_profile.java_id.is_some()
                {
                    continue;
                }

                self.app
                    .prisma_client
                    .java_profile()
                    .update(
                        crate::db::java_profile::name::equals(system_profile.name),
                        vec![crate::db::java_profile::java::connect(
                            crate::db::java::id::equals(id.clone()),
                        )],
                    )
                    .exec()
                    .await?;
            }
            self.app.invalidate(GET_JAVA_PROFILES, None);
        }

        Ok(Some(java))
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

        let profiles_in_db = app
            .prisma_client
            .java_profile()
            .find_many(vec![crate::db::java_profile::is_system_profile::equals(
                true,
            )])
            .exec()
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
        let count = app.prisma_client.java().count(vec![]).exec().await.unwrap();
        assert_eq!(count, 1);

        let from_db = app
            .prisma_client
            .java()
            .find_first(vec![])
            .exec()
            .await
            .unwrap()
            .unwrap();

        assert!(std::path::Path::new(&from_db.path).exists());

        let result_first_delete = app
            .java_manager()
            .delete_java_version(from_db.id.clone())
            .await;

        assert!(result_first_delete.is_err());

        app.prisma_client
            .app_configuration()
            .update(
                crate::db::app_configuration::id::equals(0),
                vec![crate::db::app_configuration::auto_manage_java_system_profiles::set(false)],
            )
            .exec()
            .await
            .unwrap();

        let result_second_delete = app
            .java_manager()
            .delete_java_version(from_db.id.clone())
            .await;

        assert!(result_second_delete.is_ok());

        let count = app.prisma_client.java().count(vec![]).exec().await.unwrap();
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
