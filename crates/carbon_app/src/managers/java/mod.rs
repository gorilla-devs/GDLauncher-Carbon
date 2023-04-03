use self::{discovery::Discovery, java_checker::JavaChecker};

use super::{AppInner, ManagerRef};
use crate::{
    api::keys::java::GET_AVAILABLE,
    db::{read_filters::StringFilter, PrismaClient},
    domain::java::{JavaComponent, JavaComponentType, JavaVersion},
};
use std::{
    path::PathBuf,
    sync::{Arc, Weak},
};

mod auto_setup;
mod constants;
mod discovery;
mod java_checker;
mod parser;
mod utils;

pub(crate) struct JavaManager {}

async fn get_java_component_from_db(db: &PrismaClient, path: String) -> anyhow::Result<Option<()>> {
    db.java()
        .find_unique(crate::db::java::UniqueWhereParam::PathEquals(path))
        .exec()
        .await?;

    Ok(Some(()))
}

async fn add_java_component_to_db(
    db: &Arc<PrismaClient>,
    java_component: JavaComponent,
) -> anyhow::Result<()> {
    db.java()
        .create(
            java_component.path,
            java_component.version.major as i32,
            java_component.version.try_into().unwrap(),
            java_component._type.into(),
            Into::<&str>::into(java_component.arch).to_string(),
            vec![],
        )
        .exec()
        .await?;

    Ok(())
}

async fn update_java_component_in_db_to_invalid(
    db: &Arc<PrismaClient>,
    path: String,
) -> anyhow::Result<()> {
    db.java()
        .update(
            crate::db::java::UniqueWhereParam::PathEquals(path),
            vec![crate::db::java::SetParam::SetIsValid(false)],
        )
        .exec()
        .await?;

    Ok(())
}

impl JavaManager {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn scan_and_sync<T, G>(
        app: Weak<AppInner>,
        discovery: &T,
        java_checker: &G,
    ) -> anyhow::Result<()>
    where
        T: Discovery,
        G: JavaChecker,
    {
        let db = app.upgrade().unwrap().prisma_client.clone();
        let mut local_javas = discovery.find_java_paths().await;
        let default_javas = db.default_java().find_many(vec![]).exec().await?;
        local_javas.push(PathBuf::from("java"));

        //--------------------------
        // Loop over local javas
        //--------------------------

        for local_java in local_javas {
            // Verify whether the java is valid
            let java_bin_info = java_checker
                .get_bin_info(&local_java, JavaComponentType::Local)
                .await;
            match java_bin_info {
                // If it is valid, check whether it's in the DB
                Ok(java_component) => {
                    let java = get_java_component_from_db(&db, java_component.path.clone()).await?;

                    match java {
                        // If it is in the db, update it to valid. Also make sure the version is in sync. If Major is not in sync, that is a problem
                        Some(_) => {}
                        // If it isn't in the db, add it
                        None => {
                            add_java_component_to_db(&db, java_component).await?;
                        }
                    }
                }
                // If it isn't valid, check whether it's in the DB
                Err(e) => {
                    let java =
                        get_java_component_from_db(&db, local_java.display().to_string()).await?;
                    // If it is in the db, update it to invalid
                    if java.is_some() {
                        update_java_component_in_db_to_invalid(
                            &db,
                            local_java.display().to_string(),
                        )
                        .await?;
                    }
                }
            }
        }

        // Cleanup unscanned local javas (if they are not default)
        let local_javas_from_db = db
            .java()
            .find_many(vec![crate::db::java::WhereParam::Type(
                StringFilter::Equals(JavaComponentType::Local.into()),
            )])
            .exec()
            .await?;

        for local_java_from_db in local_javas_from_db {
            if !default_javas
                .iter()
                .any(|default_java| local_java_from_db.path == default_java.path)
            {
                db.java()
                    .delete(crate::db::java::UniqueWhereParam::PathEquals(
                        local_java_from_db.path,
                    ))
                    .exec()
                    .await?;
            }
        }

        //--------------------------
        // Loop over custom javas
        //--------------------------

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
