use prisma_client_rust::{prisma_errors::query_engine::UniqueKeyViolation, QueryError};
use strum::IntoEnumIterator;

use self::{discovery::Discovery, java_checker::JavaChecker, managed::ManagedService};

use super::ManagerRef;
use crate::{
    api::keys::java::GET_SYSTEM_JAVA_PROFILES,
    db::PrismaClient,
    domain::java::{Java, JavaComponentType, SystemJavaProfile, SystemJavaProfileName},
};
use std::{
    collections::HashMap,
    path::{Component, PathBuf},
    sync::Arc,
};

mod constants;
pub mod discovery;
pub mod java_checker;
pub mod managed;
mod parser;
mod scan_and_sync;
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
        for profile in SystemJavaProfileName::iter() {
            let creation: Result<crate::db::java_system_profile::Data, QueryError> = db_client
                .java_system_profile()
                .create(profile.to_string(), vec![])
                .exec()
                .await;

            match creation {
                Err(error) if error.is_prisma_error::<UniqueKeyViolation>() => {
                    // Good, already exists
                }
                Err(error) => {
                    return Err(error.into());
                }
                Ok(_) => {
                    // Good, created
                }
            }
        }

        Ok(())
    }

    pub async fn scan_and_sync<T, G>(
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
        scan_and_sync::scan_and_sync_managed(db, java_checker).await?;

        scan_and_sync::sync_system_java_profiles(db).await?;

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

    pub async fn get_system_java_profiles(&self) -> anyhow::Result<Vec<SystemJavaProfile>> {
        let db = &self.app.prisma_client;
        let all_profiles = db
            .java_system_profile()
            .find_many(vec![])
            .exec()
            .await?
            .into_iter()
            .map(SystemJavaProfile::try_from)
            .collect::<anyhow::Result<Vec<_>>>()?;

        Ok(all_profiles)
    }

    pub async fn update_system_java_profile_path(
        &self,
        profile_name: SystemJavaProfileName,
        java_id: String,
    ) -> anyhow::Result<()> {
        let auto_manage_java = self.app.settings_manager().get().await?.auto_manage_java;

        if auto_manage_java {
            anyhow::bail!("Auto manage java is enabled");
        }

        self.app
            .prisma_client
            .java_system_profile()
            .update(
                crate::db::java_system_profile::UniqueWhereParam::NameEquals(
                    profile_name.to_string(),
                ),
                vec![crate::db::java_system_profile::SetParam::ConnectJava(
                    crate::db::java::UniqueWhereParam::IdEquals(java_id),
                )],
            )
            .exec()
            .await?;

        self.app.invalidate(GET_SYSTEM_JAVA_PROFILES, None);

        Ok(())
    }

    pub async fn delete_java_version(&self, java_id: String) -> anyhow::Result<()> {
        let auto_manage_java = self.app.settings_manager().get().await?.auto_manage_java;

        if auto_manage_java {
            anyhow::bail!("Auto manage java is enabled");
        }

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

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::{
        domain::java::{JavaArch, JavaOs, JavaVendor},
        setup_managers_for_test,
    };

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

        assert!(!result_first_delete.is_ok());

        app.prisma_client
            .app_configuration()
            .update(
                crate::db::app_configuration::id::equals(0),
                vec![crate::db::app_configuration::auto_manage_java::set(false)],
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
