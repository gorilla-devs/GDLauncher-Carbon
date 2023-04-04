use std::{
    path::{Path, PathBuf},
    sync::{Arc, Weak},
};

use crate::{
    db::{read_filters::StringFilter, PrismaClient},
    domain::java::{JavaComponent, JavaComponentType},
    managers::AppInner,
};

use super::{discovery::Discovery, java_checker::JavaChecker};

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

pub async fn scan_and_sync_local<T, G>(
    db: &Arc<PrismaClient>,
    discovery: &T,
    java_checker: &G,
) -> anyhow::Result<()>
where
    T: Discovery,
    G: JavaChecker,
{
    let mut local_javas = discovery.find_java_paths().await;
    local_javas.push(PathBuf::from("java"));
    let default_javas = db.default_java().find_many(vec![]).exec().await?;

    for local_java in local_javas {
        // Verify whether the java is valid
        let java_bin_info = java_checker
            .get_bin_info(&local_java, JavaComponentType::Local)
            .await;
        match java_bin_info {
            // If it is valid, check whether it's in the DB
            Ok(java_component) => {
                let java = get_java_component_from_db(db, java_component.path.clone()).await?;

                match java {
                    // If it is in the db, update it to valid. Also make sure the version is in sync. If Major is not in sync, that is a problem
                    Some(_) => {}
                    // If it isn't in the db, add it
                    None => {
                        add_java_component_to_db(db, java_component).await?;
                    }
                }
            }
            // If it isn't valid, check whether it's in the DB
            Err(e) => {
                let java = get_java_component_from_db(db, local_java.display().to_string()).await?;
                // If it is in the db, update it to invalid
                if java.is_some() {
                    update_java_component_in_db_to_invalid(db, local_java.display().to_string())
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

    Ok(())
}

pub async fn scan_and_sync_custom<G>(db: &Arc<PrismaClient>, java_checker: &G) -> anyhow::Result<()>
where
    G: JavaChecker,
{
    let custom_javas = db
        .java()
        .find_many(vec![crate::db::java::WhereParam::Type(
            StringFilter::Equals(JavaComponentType::Local.into()),
        )])
        .exec()
        .await?;

    for custom_java in custom_javas {
        let java_bin_info = java_checker
            .get_bin_info(
                &PathBuf::from(custom_java.path.clone()),
                JavaComponentType::Custom,
            )
            .await;

        if java_bin_info.is_err() {
            update_java_component_in_db_to_invalid(db, custom_java.path).await?;
        }
    }

    Ok(())
}
