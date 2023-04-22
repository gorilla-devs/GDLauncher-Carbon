use self::{discovery::Discovery, java_checker::JavaChecker};

use super::ManagerRef;
use crate::{
    db::PrismaClient,
    domain::java::{Java, JavaVersion},
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

    pub async fn get_default_javas(self) -> anyhow::Result<HashMap<u8, String>> {
        let db = &self.app.prisma_client;
        let all_javas = db
            .default_java()
            .find_many(vec![])
            .exec()
            .await?
            .into_iter()
            .map(|j| (j.major as u8, j.path))
            .collect();

        Ok(all_javas)
    }
}
