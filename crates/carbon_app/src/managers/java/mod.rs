use self::{discovery::Discovery, java_checker::JavaChecker};

use super::{AppInner, ManagerRef};
use crate::{
    db::{read_filters::StringFilter, PrismaClient},
    domain::java::{JavaComponent, JavaComponentType, JavaVersion},
};
use std::{
    path::PathBuf,
    sync::{Arc, Weak},
};

mod auto_setup;
mod constants;
pub mod discovery;
pub mod java_checker;
mod parser;
mod scan_and_sync;
mod utils;

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
    // pub async fn get_available_javas(self) -> anyhow::Result<HashMap<u8, JavaComponent>> {
    //     let db = &self.app.prisma_client;
    //     let all_javas = db
    //         .java()
    //         .find_many(vec![])
    //         .exec()
    //         .await?
    //         .into_iter()
    //         .map(JavaComponent::from)
    //         .map(|java| (java.version.major, java))
    //         .collect();

    //     Ok(all_javas)
    // }

    // pub async fn get_default_javas(self) -> anyhow::Result<HashMap<u8, String>> {
    //     let db = &self.app.prisma_client;
    //     let all_javas = db
    //         .default_java()
    //         .find_many(vec![])
    //         .exec()
    //         .await?
    //         .into_iter()
    //         .map(|j| (j.major as u8, j.path))
    //         .collect();

    //     Ok(all_javas)
    // }
}

// impl From<crate::db::java::Data> for JavaComponent {
//     fn from(java: crate::db::java::Data) -> Self {
//         Self {
//             path: java.path,
//             arch: JavaArch::from(&*java.arch),
//             _type: JavaComponentType::from(&*java.r#type),
//             version: JavaVersion::from(&*java.full_version),
//         }
//     }
// }
