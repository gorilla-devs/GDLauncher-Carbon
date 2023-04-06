use std::{path::PathBuf, sync::Arc};

use crate::{
    db::{read_filters::StringFilter, PrismaClient},
    domain::java::{JavaComponent, JavaComponentType},
};

use super::{discovery::Discovery, java_checker::JavaChecker};

async fn get_java_component_from_db(db: &PrismaClient, path: String) -> anyhow::Result<Option<()>> {
    let res = db
        .java()
        .find_unique(crate::db::java::UniqueWhereParam::PathEquals(path))
        .exec()
        .await?;

    Ok(if res.is_some() { Some(()) } else { None })
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
    let local_javas = discovery.find_java_paths().await;
    let default_javas = db.default_java().find_many(vec![]).exec().await?;

    for local_java in &local_javas {
        // Verify whether the java is valid
        let java_bin_info = java_checker
            .get_bin_info(local_java, JavaComponentType::Local)
            .await;
        match java_bin_info {
            // If it is valid, check whether it's in the DB
            Ok(java_component) => {
                let java = get_java_component_from_db(db, java_component.path.clone()).await?;

                match java {
                    // If it is in the db, update it to valid. Also make sure the version is in sync. If Major is not in sync, that is a problem
                    Some(_) => {
                        // TODO
                    }
                    // If it isn't in the db, add it
                    None => {
                        add_java_component_to_db(db, java_component).await?;
                    }
                }
            }
            // If it isn't valid, check whether it's in the DB
            Err(_) => {
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
            && !local_javas
                .iter()
                .any(|local_java| local_java_from_db.path == local_java.display().to_string())
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

#[cfg(test)]
mod test {
    use crate::{
        domain::java::{JavaArch, JavaComponent, JavaComponentType, JavaVersion},
        managers::java::{
            discovery::MockDiscovery,
            java_checker::{MockJavaChecker, MockJavaCheckerInvalid},
            scan_and_sync::{add_java_component_to_db, scan_and_sync_custom, scan_and_sync_local},
        },
        setup_managers_for_test,
    };

    #[tokio::test]
    async fn test_scan_and_sync_local() {
        let app = setup_managers_for_test().await;
        let db = &app.prisma_client;

        let discovery = &MockDiscovery;
        let java_checker = &MockJavaChecker;
        // // Insert one already existing path (/usr/bin/java) and one that should not exist anymore, hence removed (/usr/bin/java2)

        let component_to_remove = JavaComponent {
            path: "/usr/bin/java2".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_64,
        };
        add_java_component_to_db(db, component_to_remove)
            .await
            .unwrap();

        let component_to_keep = JavaComponent {
            path: "/usr/bin/java".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_64,
        };

        add_java_component_to_db(db, component_to_keep)
            .await
            .unwrap();

        scan_and_sync_local(db, discovery, java_checker)
            .await
            .unwrap();

        // Ensure /usr/bin/java is still there but /usr/bin/java2 is gone. Also ensure /opt/java/bin/java and
        // /opt/homebrew/opt/openjdk/bin/java" are added

        let java_components = db.java().find_many(vec![]).exec().await.unwrap();

        assert_eq!(java_components.len(), 3);
    }

    #[tokio::test]
    async fn test_scan_and_sync_local_to_invalid() {
        let app = setup_managers_for_test().await;
        let db = &app.prisma_client;
        let discovery = &MockDiscovery;
        let java_checker = &MockJavaCheckerInvalid;

        let component_to_add = JavaComponent {
            path: "/usr/bin/java".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Local,
            arch: JavaArch::X86_64,
        };

        add_java_component_to_db(db, component_to_add)
            .await
            .unwrap();

        scan_and_sync_local(db, discovery, java_checker)
            .await
            .unwrap();

        let java_components = db.java().find_many(vec![]).exec().await.unwrap();

        // Since the db only contains one component, it should be set as invalid.
        // The other 2, since they don't already exist, are not added nor updated.

        assert_eq!(java_components.len(), 1);
        assert!(!java_components[0].is_valid);
    }

    #[tokio::test]
    async fn test_scan_and_sync_custom_to_invalid() {
        let app = setup_managers_for_test().await;
        let db = &app.prisma_client;
        let java_checker = &MockJavaCheckerInvalid;

        let component_to_add = JavaComponent {
            path: "/my/custom/path".to_string(),
            version: JavaVersion::from_major(8),
            _type: JavaComponentType::Custom,
            arch: JavaArch::X86_64,
        };

        add_java_component_to_db(db, component_to_add)
            .await
            .unwrap();

        scan_and_sync_custom(db, java_checker).await.unwrap();

        let java_components = db.java().find_many(vec![]).exec().await.unwrap();

        // Since the db only contains one component, it should be set as invalid.
        // The other 2, since they don't already exist, are not added nor updated.

        assert_eq!(java_components.len(), 1);
        assert!(!java_components[0].is_valid);
    }
}
