use anyhow::Ok;
use strum::IntoEnumIterator;

use self::{discovery::Discovery, java_checker::JavaChecker, managed::ManagedService};

use super::ManagerRef;
use crate::{
    api::keys::java::GET_SYSTEM_JAVA_PROFILES,
    db::PrismaClient,
    domain::java::{Java, SystemJavaProfile, SystemJavaProfileName},
};
use std::{collections::HashMap, sync::Arc};

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
        if db_client.java_system_profile().count(vec![]).exec().await? == 0 {
            for profile in SystemJavaProfileName::iter() {
                db_client
                    .java_system_profile()
                    .create(profile.to_string(), vec![])
                    .exec()
                    .await?;
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

        if !auto_manage_java {
            anyhow::bail!("Auto manage java is disabled");
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
}
