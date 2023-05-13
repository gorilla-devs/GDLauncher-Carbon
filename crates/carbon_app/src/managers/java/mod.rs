use self::{discovery::Discovery, java_checker::JavaChecker};

use super::ManagerRef;
use crate::{
    db::PrismaClient,
    domain::java::{Java, SystemProfile, SYSTEM_PROFILES},
};
use std::{collections::HashMap, sync::Arc};

mod auto_setup;
mod constants;
pub mod discovery;
pub mod java_checker;
mod parser;
mod scan_and_sync;
pub mod utils;

pub(crate) struct JavaManager {}

impl JavaManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn ensure_profiles_in_db(db_client: &PrismaClient) -> anyhow::Result<()> {
        if db_client.java_system_profile().count(vec![]).exec().await? == 0 {
            for profile in SYSTEM_PROFILES {
                db_client
                    .java_system_profile()
                    .create(String::from(profile), vec![])
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

        Ok(())
    }
}

impl ManagerRef<'_, JavaManager> {
    pub async fn get_available_javas(self) -> anyhow::Result<HashMap<u8, Vec<Java>>> {
        let db = &self.app.prisma_client;
        let all_javas = db.java().find_many(vec![]).exec().await?;

        let mut result = HashMap::new();

        for java in all_javas {
            let major_version = java.major as u8;
            let javas = result.entry(major_version).or_insert_with(Vec::new);
            javas.push(Java::from(java));
        }

        Ok(result)
    }

    pub async fn get_java_profiles(
        self,
    ) -> anyhow::Result<Vec<crate::db::java_system_profile::Data>> {
        let db = &self.app.prisma_client;
        let all_profiles = db
            .java_system_profile()
            .find_many(vec![])
            .exec()
            .await?
            .into_iter()
            .collect();

        Ok(all_profiles)
    }
}
